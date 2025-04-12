// Written by Alberto Ruiz 2024-03-08
// This is the HTTP server module, it will handle the requests and responses
// Also provide utilities to parse the requests and build the responses

pub mod brew;
mod methods;
mod request;
mod response;
mod status;

use self::response::EmptyHttpResponse;
use self::response::HttpResponseCommon;
use self::response::IterError;

pub use self::methods::HttpMethod;
pub use self::request::HttpRequest;
use self::request::HttpRequestBuilder;
pub use self::response::{HttpResponse, StreamedResponse};
pub use self::status::HttpStatus;

use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const BUFFER_SIZE: usize = 1024 * 2;
const KEEP_ALIVE_TTL: Duration = Duration::from_secs(10);

#[macro_export]
macro_rules! headers {
    ( $($k:expr => $v:expr),*) => {
        {
            use std::collections::HashMap;
            let mut headers: HashMap<String, String> = HashMap::new();
            $( headers.insert($k.to_string(), $v.to_string()); )*
            Some(headers)
        }
    };
}

pub struct Hteapot {
    port: u16,
    address: String,
    threads: u16,
}

struct SocketStatus {
    ttl: Instant,
    reading: bool,
    write: bool,
    response: Box<dyn HttpResponseCommon>,
    request: HttpRequestBuilder,
    index_writed: usize,
}

struct SocketData {
    stream: TcpStream,
    status: Option<SocketStatus>,
}

impl Hteapot {
    // Constructor
    pub fn new(address: &str, port: u16) -> Self {
        Hteapot {
            port,
            address: address.to_string(),
            threads: 1,
        }
    }

    pub fn new_threaded(address: &str, port: u16, threads: u16) -> Self {
        Hteapot {
            port,
            address: address.to_string(),
            threads: if threads == 0 { 1 } else { threads },
        }
    }

    // Start the server
    pub fn listen(
        &self,
        action: impl Fn(HttpRequest) -> Box<dyn HttpResponseCommon> + Send + Sync + 'static,
    ) {
        let addr = format!("{}:{}", self.address, self.port);
        let listener = TcpListener::bind(addr);
        let listener = match listener {
            Ok(listener) => listener,
            Err(e) => {
                eprintln!("Error L: {}", e);
                return;
            }
        };

        let pool: Arc<(Mutex<VecDeque<TcpStream>>, Condvar)> =
            Arc::new((Mutex::new(VecDeque::new()), Condvar::new()));
        let priority_list: Arc<Mutex<Vec<usize>>> =
            Arc::new(Mutex::new(vec![0; self.threads as usize]));
        let arc_action = Arc::new(action);

        for thread_index in 0..self.threads {
            let pool_clone = pool.clone();
            let action_clone = arc_action.clone();
            let priority_list_clone = priority_list.clone();

            thread::spawn(move || {
                let mut streams_to_handle = Vec::new();
                loop {
                    {
                        let (lock, cvar) = &*pool_clone;
                        let mut pool = lock.lock().expect("Error locking pool");

                        if streams_to_handle.is_empty() {
                            pool = cvar
                                .wait_while(pool, |pool| pool.is_empty())
                                .expect("Error waiting on cvar");
                        }

                        while let Some(stream) = pool.pop_back() {
                            let socket_status = SocketStatus {
                                ttl: Instant::now(),
                                reading: true,
                                write: false,
                                response: Box::new(EmptyHttpResponse {}),
                                request: HttpRequestBuilder::new(),
                                index_writed: 0,
                            };
                            let socket_data = SocketData {
                                stream,
                                status: Some(socket_status),
                            };
                            streams_to_handle.push(socket_data);
                        }
                    }

                    {
                        let mut priority_list = priority_list_clone
                            .lock()
                            .expect("Error locking priority list");
                        priority_list[thread_index as usize] = streams_to_handle.len();
                    }

                    streams_to_handle.retain_mut(|s| {
                        if s.status.is_none() {
                            return false;
                        }
                        Hteapot::handle_client(s, &action_clone).is_some()
                    });
                }
            });
        }

        loop {
            let stream = listener.accept();
            if stream.is_err() {
                continue;
            }
            let (stream, _) = stream.unwrap();
            stream
                .set_nonblocking(true)
                .expect("Error setting non-blocking");
            stream.set_nodelay(true).expect("Error setting no delay");

            {
                let (lock, cvar) = &*pool;
                let mut pool = lock.lock().expect("Error locking pool");

                // Add the connection to the pool for the least-loaded thread
                pool.push_front(stream);
                cvar.notify_one();
            }
        }
    }

    fn handle_client(
        socket_data: &mut SocketData,
        action: &Arc<impl Fn(HttpRequest) -> Box<dyn HttpResponseCommon> + Send + Sync + 'static>,
    ) -> Option<()> {
        let status = socket_data.status.as_mut()?;

        // Fix by miky-rola 2025-04-08
        // Check if the TTL (time-to-live) for the connection has expired.
        // If the connection is idle for longer than `KEEP_ALIVE_TTL` and no data is being written,
        // the connection is gracefully shut down to free resources.
        if Instant::now().duration_since(status.ttl) > KEEP_ALIVE_TTL && !status.write {
            let _ = socket_data.stream.shutdown(Shutdown::Both);
            return None;
        }
        // If the request is not yet complete, read data from the stream into a buffer.
        // This ensures that the server can handle partial or chunked requests.
        if !status.request.done {
            let mut buffer = [0; BUFFER_SIZE];
            match socket_data.stream.read(&mut buffer) {
                Err(e) => match e.kind() {
                    io::ErrorKind::WouldBlock => {
                        return Some(());
                    }
                    io::ErrorKind::ConnectionReset => {
                        return None;
                    }
                    _ => {
                        eprintln!("Read error: {:?}", e);
                        return None;
                    }
                },
                Ok(m) => {
                    if m == 0 {
                        return None;
                    }
                    status.ttl = Instant::now();
                    let r = status.request.append(buffer[..m].to_vec());
                    if r.is_err() {
                        // Early return response if not valid request is sended
                        let error_msg = r.err().unwrap();
                        let response =
                            HttpResponse::new(HttpStatus::BadRequest, error_msg, None).to_bytes();
                        let _ = socket_data.stream.write(&response);
                        let _ = socket_data.stream.flush();
                        let _ = socket_data.stream.shutdown(Shutdown::Both);
                        return None;
                    }
                }
            }
        }

        let request = status.request.get();
        if request.is_none() {
            return Some(());
        }
        let request = request.unwrap();

        let keep_alive = request
            .headers
            .get("connection") //all headers are turn lowercase in the builder
            .map(|v| v.to_lowercase() == "keep-alive")
            .unwrap_or(false);
        if !status.write {
            let mut response = action(request);
            if keep_alive {
                response
                    .base()
                    .headers
                    .entry("Connection".to_string())
                    .or_insert("keep-alive".to_string());
                response.base().headers.insert(
                    "Keep-Alive".to_string(),
                    format!("timeout={}", KEEP_ALIVE_TTL.as_secs()),
                );
            } else {
                response
                    .base()
                    .headers
                    .insert("Connection".to_string(), "close".to_string());
            }
            status.write = true;
            status.response = response;
        }

        // Write the response to the client in chunks using the `peek` and `next` methods.
        // This ensures that large responses are sent incrementally without blocking the server.
        loop {
            match status.response.peek() {
                Ok(n) => match socket_data.stream.write(&n) {
                    Ok(_) => {
                        status.ttl = Instant::now();
                        let _ = status.response.next();
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        return Some(());
                    }
                    Err(e) => {
                        eprintln!("Write error: {:?}", e);
                        return None;
                    }
                },
                Err(IterError::WouldBlock) => {
                    status.ttl = Instant::now();
                    return Some(());
                }
                Err(_) => break,
            }
        }

        if keep_alive {
            status.reading = true;
            status.write = false;
            status.index_writed = 0;
            status.request = HttpRequestBuilder::new();
            return Some(());
        } else {
            let _ = socket_data.stream.shutdown(Shutdown::Both);
            None
        }
    }
}

#[cfg(test)]
#[test]
fn test_http_response_maker() {
    let mut response = HttpResponse::new(HttpStatus::IAmATeapot, "Hello, World!", None);
    let response = String::from_utf8(response.to_bytes()).unwrap();
    let expected_response = format!(
        "HTTP/1.1 418 I'm a teapot\r\nContent-Length: 13\r\nServer: HTeaPot/{}\r\n\r\nHello, World!\r\n",
        VERSION
    );
    let expected_response_list = expected_response.split("\r\n");
    for item in expected_response_list.into_iter() {
        assert!(response.contains(item));
    }
}

#[cfg(test)]
#[test]
fn test_keep_alive_connection() {
    let mut response = HttpResponse::new(
        HttpStatus::OK,
        "Keep-Alive Test",
        headers! {
            "Connection" => "keep-alive",
            "Content-Length" => "15"
        },
    );

    response.base().headers.insert(
        "Keep-Alive".to_string(),
        format!("timeout={}", KEEP_ALIVE_TTL.as_secs()),
    );

    let response_bytes = response.to_bytes();
    let response_str = String::from_utf8(response_bytes.clone()).unwrap();

    assert!(response_str.contains("HTTP/1.1 200 OK"));
    assert!(response_str.contains("Content-Length: 15"));
    assert!(response_str.contains("Connection: keep-alive"));
    assert!(response_str.contains("Keep-Alive: timeout=10"));
    assert!(response_str.contains("Server: HTeaPot/"));
    assert!(response_str.contains("Keep-Alive Test"));

    let mut second_response = HttpResponse::new(
        HttpStatus::OK,
        "Second Request",
        headers! {
            "Connection" => "keep-alive",
            "Content-Length" => "14" // Length for "Second Request"
        },
    );

    second_response.base().headers.insert(
        "Keep-Alive".to_string(),
        format!("timeout={}", KEEP_ALIVE_TTL.as_secs()),
    );

    let second_response_bytes = second_response.to_bytes();
    let second_response_str = String::from_utf8(second_response_bytes.clone()).unwrap();

    assert!(second_response_str.contains("HTTP/1.1 200 OK"));
    assert!(second_response_str.contains("Content-Length: 14"));
    assert!(second_response_str.contains("Connection: keep-alive"));
    assert!(second_response_str.contains("Keep-Alive: timeout=10"));
    assert!(second_response_str.contains("Server: HTeaPot/"));
    assert!(second_response_str.contains("Second Request"));
}
