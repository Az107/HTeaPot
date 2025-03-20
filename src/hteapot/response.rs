use super::HttpStatus;
use super::{BUFFER_SIZE, VERSION};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

pub struct BaseResponse {
    pub status: HttpStatus,
    pub headers: HashMap<String, String>,
}

impl BaseResponse {
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

pub struct HttpResponse {
    base: BaseResponse,
    pub content: Vec<u8>,
    raw: Option<Vec<u8>>,
    is_raw: bool,
    index: usize,
}

pub trait HttpResponseCommon {
    fn base(&mut self) -> &mut BaseResponse;
    fn next(&mut self) -> Result<Vec<u8>, IterError>;
    fn peek(&mut self) -> Result<Vec<u8>, IterError>;
}

#[derive(Debug)]
pub enum IterError {
    WouldBlock,
    Finished,
}

impl HttpResponse {
    pub fn new<B: AsRef<[u8]>>(
        status: HttpStatus,
        content: B,
        headers: Option<HashMap<String, String>>,
    ) -> Box<Self> {
        let mut headers = headers.unwrap_or(HashMap::new());
        let content = content.as_ref();
        headers.insert("Content-Length".to_string(), content.len().to_string());
        headers.insert(
            "Server".to_string(),
            format!("HTeaPot/{}", VERSION).to_string(),
        );
        Box::new(HttpResponse {
            base: BaseResponse { status, headers },
            content: content.to_owned(),
            raw: None,
            is_raw: false,
            index: 0,
        })
    }

    pub fn new_raw(raw: Vec<u8>) -> Self {
        HttpResponse {
            base: BaseResponse {
                status: HttpStatus::IAmATeapot,
                headers: HashMap::new(),
            },
            content: vec![],
            raw: Some(raw),
            is_raw: true,
            index: 0,
        }
    }

    pub fn is_raw(&self) -> bool {
        self.is_raw
    }

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
        // println!("{}/{}",self.)
        let byte_chunk = raw.next().ok_or(IterError::Finished)?.to_vec();
        return Ok(byte_chunk);
    }
}

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

pub struct StreamedResponse {
    base: BaseResponse,
    receiver: Receiver<Vec<u8>>,
    has_end: Arc<AtomicBool>,
    join_handle: JoinHandle<()>,
}

impl StreamedResponse {
    pub fn new(action: impl Fn(Sender<Vec<u8>>) + Send + Sync + 'static) -> Box<Self> {
        let action = Arc::new(action);
        let (tx, rx) = mpsc::channel();
        let action_clon = action.clone();
        let mut base = BaseResponse {
            status: HttpStatus::OK,
            headers: HashMap::new(),
        };
        base.headers
            .insert("Transfer-Encoding".to_string(), "chunked".to_string());
        base.headers.insert(
            "Server".to_string(),
            format!("HTeaPot/{}", VERSION).to_string(),
        );
        let _ = tx.send(base.to_bytes());
        let has_end = Arc::new(AtomicBool::new(false));
        let has_end_clone = has_end.clone();
        let jh = thread::spawn(move || {
            action_clon(tx);
            println!("Ended!");
            has_end_clone.store(true, Ordering::SeqCst);
        });
        Box::new(StreamedResponse {
            base,
            has_end,
            receiver: rx,
            join_handle: jh,
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
        if self.has_end() {
            return Err(IterError::Finished);
        }
        self.receiver
            .recv_timeout(Duration::from_millis(100))
            .map_err(|_| IterError::WouldBlock)
    }

    fn peek(&mut self) -> Result<Vec<u8>, IterError> {
        if self.has_end() {
            return Err(IterError::Finished);
        }
        self.receiver
            .recv_timeout(Duration::from_millis(100))
            .map_err(|_| IterError::WouldBlock)
    }
}
