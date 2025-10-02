//! HTTP response types for HTeaPot.
//!
//! Supports multiple types of responses:
//! - [`HttpResponse`] for standard fixed-size responses
//! - [`StreamedResponse`] for chunked transfer encoding
//! - [`EmptyHttpResponse`] as a sentinel or fallback
//!
//! All response types implement the [`HttpResponseCommon`] trait.

use super::http::Headers;

use super::HttpStatus;
use super::{BUFFER_SIZE, VERSION};
use std::collections::VecDeque;
use std::io::Write;
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SendError, Sender, TryRecvError};
use std::thread::JoinHandle;
use std::time::Duration;
use std::{io, thread};

/// Basic HTTP status line + headers.
#[derive(Clone)]
pub struct BaseResponse {
    pub status: HttpStatus,
    pub headers: Headers,
}

impl BaseResponse {
    /// Converts the status + headers into a properly formatted HTTP header block.
    pub fn to_bytes(&mut self) -> Vec<u8> {
        let mut headers_text = String::new();
        for (key, value) in self.headers.iter() {
            headers_text.push_str(&format!("{}: {}\r\n", key, value));
        }

        let response_header = format!(
            "HTTP/1.1 {} {}\r\n{}\r\n",
            self.status as u16,
            self.status.to_string(),
            headers_text
        );

        let mut response = Vec::new();
        response.extend_from_slice(response_header.as_bytes());
        response
    }
}

/// Represents a full HTTP response (headers + body).
#[derive(Clone)]
pub struct HttpResponse {
    base: BaseResponse,
    pub content: Vec<u8>,
    raw: Option<Vec<u8>>,
    is_raw: bool,
    index: usize,
}

/// Trait shared by all response types (normal, streamed, etc.)
pub trait HttpResponseCommon {
    /// Returns a mutable reference to the base response (for status/headers).
    fn base(&mut self) -> &mut BaseResponse;

    /// Advances and returns the next chunk of the response body.
    fn next(&mut self) -> Result<Vec<u8>, IterError>;

    /// Advances and returns the next chunk of the response body.
    fn peek(&mut self) -> Result<Vec<u8>, IterError>;

    fn set_stream(&mut self, _stream: &TcpStream) {
        ()
    }
}

/// Error returned during response iteration.
#[derive(Debug)]
pub enum IterError {
    WouldBlock,
    Finished,
}

impl HttpResponse {
    /// Creates a standard HTTP response with body and optional headers.
    ///
    /// Automatically sets `Content-Length` and `Server` headers.
    pub fn new<B: AsRef<[u8]>>(
        status: HttpStatus,
        content: B,
        headers: Option<Headers>,
    ) -> Box<Self> {
        let mut headers = headers.unwrap_or(Headers::new());
        let content = content.as_ref();

        headers.insert("Content-Length", &content.len().to_string());
        headers.insert("Server", &format!("HTeaPot/{}", VERSION).to_string());

        Box::new(HttpResponse {
            base: BaseResponse { status, headers },
            content: content.to_owned(),
            raw: None,
            is_raw: false,
            index: 0,
        })
    }

    /// Creates a raw response from raw bytes (used for proxy responses).
    pub fn new_raw(raw: Vec<u8>) -> Self {
        HttpResponse {
            base: BaseResponse {
                status: HttpStatus::IAmATeapot,
                headers: Headers::new(),
            },
            content: vec![],
            raw: Some(raw),
            is_raw: true,
            index: 0,
        }
    }

    /// Returns true if this is a raw (proxy) response.
    pub fn is_raw(&self) -> bool {
        self.is_raw
    }

    /// Serializes the entire response into a byte buffer.
    pub fn to_bytes(&mut self) -> Vec<u8> {
        if self.is_raw() {
            return self.raw.clone().unwrap();
        }

        let mut headers_text = String::new();
        for (key, value) in self.base.headers.iter() {
            headers_text.push_str(&format!("{}: {}\r\n", key, value));
        }

        let response_header = format!(
            "HTTP/1.1 {} {}\r\n{}\r\n",
            self.base.status as u16,
            self.base.status.to_string(),
            headers_text
        );

        let mut response = Vec::new();
        response.extend_from_slice(response_header.as_bytes());
        response.append(&mut self.content);
        response.push(0x0D); // Carriage Return
        response.push(0x0A); // Line Feed
        response
    }
}

impl HttpResponseCommon for HttpResponse {
    fn base(&mut self) -> &mut BaseResponse {
        &mut self.base
    }

    fn next(&mut self) -> Result<Vec<u8>, IterError> {
        let byte_chunk = self.peek()?;
        self.index += 1;
        return Ok(byte_chunk);
    }

    fn peek(&mut self) -> Result<Vec<u8>, IterError> {
        if self.raw.is_none() {
            self.raw = Some(self.to_bytes());
        }

        let raw = self.raw.as_ref().unwrap();
        let mut raw = raw.chunks(BUFFER_SIZE).skip(self.index);
        let byte_chunk = raw.next().ok_or(IterError::Finished)?.to_vec();
        return Ok(byte_chunk);
    }
}

/// Dummy response used when nothing needs to be returned.
pub struct EmptyHttpResponse {}

impl EmptyHttpResponse {}
impl HttpResponseCommon for EmptyHttpResponse {
    fn base(&mut self) -> &mut BaseResponse {
        panic!("Invalid state")
    }

    fn next(&mut self) -> Result<Vec<u8>, IterError> {
        Err(IterError::Finished)
    }

    fn peek(&mut self) -> Result<Vec<u8>, IterError> {
        Err(IterError::Finished)
    }
}

/// Sends response chunks in a `Transfer-Encoding: chunked` format.
pub struct ChunkSender(Sender<Vec<u8>>);

impl ChunkSender {
    /// Sends a new chunk to the output stream.
    ///
    /// Prepends the size in hex followed by CRLF, then the chunk, then another CRLF.

    // fn new(sender: Sender<Vec<u8>>) -> Self {
    //     Self(sender)
    // }
    pub fn send(&self, msg: Vec<u8>) -> Result<(), SendError<Vec<u8>>> {
        let mut response = Vec::new();
        let len_bytes = format!("{:X}\r\n", msg.len()).into_bytes();
        response.extend(len_bytes);
        response.extend(&msg);
        response.extend(b"\r\n");
        self.0.send(response)
    }

    // fn end(&self) -> Result<(), SendError<Vec<u8>>> {}
}

/// Represents a streaming HTTP response using chunked transfer encoding.
///
/// Runs the streaming action in a background thread. Chunks are sent via a channel.
pub struct StreamedResponse {
    base: BaseResponse,
    receiver: Receiver<Vec<u8>>,
    has_end: Arc<AtomicBool>,
    _join_handle: JoinHandle<()>,
    queue: VecDeque<Vec<u8>>,
}

impl StreamedResponse {
    /// Creates a new streamed response. The provided closure is run in a separate thread.
    ///
    /// The closure is given a `ChunkSender` to emit data. The response ends when the closure exits.
    pub fn new(action: impl Fn(ChunkSender) + Send + Sync + 'static) -> Box<Self> {
        let action = Arc::new(action);
        let (tx, rx) = mpsc::channel();

        let mut base = BaseResponse {
            status: HttpStatus::OK,
            headers: Headers::new(),
        };

        base.headers.insert("Transfer-Encoding", "chunked");
        base.headers
            .insert("Server", &format!("HTeaPot/{}", VERSION));

        let _ = tx.send(base.to_bytes());
        let has_end = Arc::new(AtomicBool::new(false));
        let action_clon = action.clone();
        let has_end_clone = has_end.clone();

        let jh = thread::spawn(move || {
            let chunk_sender = ChunkSender(tx.clone());
            action_clon(chunk_sender);
            let _ = tx.clone().send(b"0\r\n\r\n".to_vec());
            has_end_clone.store(true, Ordering::SeqCst);
        });

        Box::new(StreamedResponse {
            base,
            has_end,
            receiver: rx,
            _join_handle: jh,
            queue: VecDeque::new(),
        })
    }

    fn has_end(&self) -> bool {
        self.has_end.load(Ordering::SeqCst)
    }
}

impl HttpResponseCommon for StreamedResponse {
    fn base(&mut self) -> &mut BaseResponse {
        &mut self.base
    }

    fn next(&mut self) -> Result<Vec<u8>, IterError> {
        self.peek()
    }

    fn peek(&mut self) -> Result<Vec<u8>, IterError> {
        if self.queue.is_empty() {
            let r = self.receiver.try_recv().map_err(|e| match e {
                TryRecvError::Empty => IterError::WouldBlock,
                TryRecvError::Disconnected => {
                    if self.has_end() {
                        IterError::Finished
                    } else {
                        IterError::WouldBlock
                    }
                }
            })?;
            self.queue.push_back(r.clone());
            return Ok(r);
        } else {
            self.queue.pop_front().ok_or(IterError::WouldBlock)
        }
    }
}

pub struct TunnelResponse {
    base: BaseResponse,
    addr: String,
    has_end: Arc<AtomicBool>,
    stream_in: Option<TcpStream>, // In as Stream from the client *in* this server
    stream_out: Option<TcpStream>, // Out as Stream from the server *to* this server
}

impl TunnelResponse {
    pub fn new(addr: &str) -> Box<Self> {
        return Box::new(TunnelResponse {
            base: BaseResponse {
                status: HttpStatus::OK,
                headers: Headers::new(),
                // headers: headers! {"connection" => "keep-alive"}.unwrap(),
            },
            addr: addr.to_string(),
            has_end: Arc::new(AtomicBool::new(false)),
            stream_in: None,
            stream_out: None,
        });
    }
}

impl HttpResponseCommon for TunnelResponse {
    fn base(&mut self) -> &mut BaseResponse {
        &mut self.base
    }

    fn next(&mut self) -> Result<Vec<u8>, IterError> {
        self.peek()
    }

    fn peek(&mut self) -> Result<Vec<u8>, IterError> {
        if self.has_end.load(Ordering::SeqCst) {
            if let Some(sock_in) = &self.stream_in {
                let _ = sock_in.shutdown(std::net::Shutdown::Both);
            }
            if let Some(sock_out) = &self.stream_out {
                let _ = sock_out.shutdown(std::net::Shutdown::Both);
            }
            return Err(IterError::Finished);
        }
        let mut buf = [0; 1];
        let _ = self.stream_in.as_ref().unwrap().peek(&mut buf);

        return Err(IterError::WouldBlock);
    }

    fn set_stream(&mut self, stream: &TcpStream) {
        let mut client_stream = stream.try_clone().expect("clone failed...");
        self.stream_in = Some(client_stream.try_clone().expect("clone failed..."));
        let server_stream = TcpStream::connect(&self.addr);
        if server_stream.is_err() {
            println!("Error connecting");
            return;
        }
        let mut server_stream = server_stream.unwrap();
        self.stream_out = Some(server_stream.try_clone().expect("clone failed..."));
        let _ = client_stream.set_nonblocking(false);
        let _ = client_stream.set_nodelay(true);
        let _ = client_stream.set_read_timeout(Some(Duration::from_millis(500)));
        let _ = client_stream.set_write_timeout(Some(Duration::from_millis(500)));
        let _ = client_stream.write_all(&self.base.to_bytes());
        let mut server_stream_1 = server_stream.try_clone().expect("Error cloning");
        let mut client_stream_1 = client_stream.try_clone().expect("clone failed...");
        let has_ended = self.has_end.clone();
        thread::spawn(move || {
            let _ = io::copy(&mut client_stream_1, &mut server_stream_1);
            has_ended.store(true, Ordering::SeqCst);
        });

        let has_ended = self.has_end.clone();
        thread::spawn(move || {
            let _ = io::copy(&mut server_stream, &mut client_stream);
            has_ended.store(true, Ordering::SeqCst);
        });
    }
}
