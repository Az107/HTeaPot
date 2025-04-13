// Written by Alberto Ruiz 2024-03-08
//
// This is the HTTP server module, it will handle the requests and responses
// Also provides utilities to parse the requests and build the response

//! HTeaPot HTTP server core.
//!
//! This module provides a multithreaded HTTP/1.1 server built for performance and ease of use.
//! It handles request parsing, response building, connection lifecycle (keep-alive)
//! and hooks.
//!
//! Core types:
//! - [`Hteapot`] — the main server entry point
//! - [`HttpRequest`] and [`HttpResponse`] — re-exported from submodules
//!
//! Use [`Hteapot::listen`] to start a server with a request handler closure.
//! ```

/// Submodules for HTTP functionality.
pub mod brew; // HTTP client implementation
mod methods; // HTTP method and status enums
mod request; // Request parsing and builder
mod response; // Response types and streaming
mod status; // Status code mapping

// Internal types used for connection management
use self::response::{EmptyHttpResponse, HttpResponseCommon, IterError};
// use std::sync::atomic::{AtomicBool, Ordering};

// Public API exposed by this module
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

/// Crate version as set by `Cargo.toml`.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Size of the buffer used for reading from the TCP stream.
const BUFFER_SIZE: usize = 1024 * 2;

/// Time-to-live for keep-alive connections.
const KEEP_ALIVE_TTL: Duration = Duration::from_secs(10);

/// Helper macro to construct header maps.
///
/// # Example
/// ```rust
/// use hteapot::headers;
/// let headers = headers! {
///     "Content-Type" => "text/html",
///     "X-Custom" => "value"
/// };
/// ```
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
    shutdown_signal: Option<Arc<AtomicBool>>,
    shutdown_hooks: Vec<Arc<dyn Fn() + Send + Sync + 'static>>,
}

/// Represents the state of a connection's lifecycle.
struct SocketStatus {
    ttl: Instant,
    reading: bool,
    write: bool,
    response: Box<dyn HttpResponseCommon>,
    request: HttpRequestBuilder,
    index_writed: usize,
}

/// Wraps a TCP stream and its associated state.
struct SocketData {
    stream: TcpStream,
    status: Option<SocketStatus>,
}

impl Hteapot {
    pub fn set_shutdown_signal(&mut self, signal: Arc<AtomicBool>) {
        self.shutdown_signal = Some(signal);
    }

    pub fn add_shutdown_hook<F>(&mut self, hook: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.shutdown_hooks.push(Arc::new(hook));
    }

    // Constructor
    pub fn new(address: &str, port: u16) -> Self {
        Hteapot {
            port,
            address: address.to_string(),
            threads: 1,
            shutdown_signal: None,
            shutdown_hooks: Vec::new(),
        }
    }

    pub fn new_threaded(address: &str, port: u16, threads: u16) -> Self {
        Hteapot {
            port,
            address: address.to_string(),
            threads: if threads == 0 { 1 } else { threads },
            shutdown_signal: None,
            shutdown_hooks: Vec::new(),
        }
    }

    // Start the server
    pub fn listen(
        &self,
        action: impl Fn(HttpRequest) -> Box<dyn HttpResponseCommon> + Send + Sync + 'static,
    ) {
        let addr = format!("{}:{}", self.address, self.port);
        let listener = match TcpListener::bind(addr) {
            Ok(listener) => listener,
            Err(e) => {
                eprintln!("Error binding to address: {}", e);
                return;
            }
        };

        let pool: Arc<(Mutex<VecDeque<TcpStream>>, Condvar)> =
            Arc::new((Mutex::new(VecDeque::new()), Condvar::new()));
        let priority_list: Arc<Mutex<Vec<usize>>> =
            Arc::new(Mutex::new(vec![0; self.threads as usize]));
        let arc_action = Arc::new(action);

        // Clone shutdown_signal and share the shutdown_hooks via Arc
        let shutdown_signal = self.shutdown_signal.clone();
        let shutdown_hooks = Arc::new(self.shutdown_hooks.clone());

        for thread_index in 0..self.threads {
            let pool_clone = pool.clone();
            let action_clone = arc_action.clone();
            let priority_list_clone = priority_list.clone();
            let shutdown_signal_clone = shutdown_signal.clone();

            thread::spawn(move || {
                let mut streams_to_handle = Vec::new();
                loop {
                    {
                        let (lock, cvar) = &*pool_clone;
                        let mut pool = lock.lock().expect("Error locking pool");
                        if streams_to_handle.is_empty() {
                            // Store the returned guard back into pool
                            pool = cvar
                                .wait_while(pool, |pool| pool.is_empty())
                                .expect("Error waiting on cvar");
                        }

                            if let Some(signal) = &shutdown_signal_clone {
                                if !signal.load(Ordering::SeqCst) {
                                    break; // Exit the server loop
                                }
                            }
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
            if let Some(signal) = &shutdown_signal {
                if !signal.load(Ordering::SeqCst) {
                    let (lock, cvar) = &*pool;
                    let _guard = lock.lock().unwrap();
                    cvar.notify_all();
                    for hook in shutdown_hooks.iter() {
                        hook();
                    }
                    break;
                }
            }
            let stream = match listener.accept() {
                Ok((stream, _)) => stream,
                Err(_) => continue,
            };

            if stream.set_nonblocking(true).is_err() {
                eprintln!("Error setting non-blocking mode on stream");
                continue;
            }
            if stream.set_nodelay(true).is_err() {
                eprintln!("Error setting no delay on stream");
                continue;
            }

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

        // Check if the TTL (time-to-live) for the connection has expired.
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
                    io::ErrorKind::WouldBlock => return Some(()),
                    io::ErrorKind::ConnectionReset => return None,
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

        let request = status.request.get()?;
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

        // Write the response to the client in chunks
        loop {
            match status.response.peek() {
                Ok(n) => match socket_data.stream.write(&n) {
                    Ok(_) => {
                        status.ttl = Instant::now();
                        let _ = status.response.next();
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => return Some(()),
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
mod tests {
    use super::*;

    #[test]
    fn test_http_response_maker() {
        let mut response = HttpResponse::new(HttpStatus::IAmATeapot, "Hello, World!", None);
        let response = String::from_utf8(response.to_bytes()).unwrap();
        let expected_response = format!(
            "HTTP/1.1 418 I'm a teapot\r\nContent-Length: 13\r\nServer: HTeaPot/{}\r\n\r\nHello, World!\r\n",
            VERSION
        );
        let expected_response_list = expected_response.split("\r\n");
        for item in expected_response_list {
            assert!(response.contains(item));
        }
    }

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
        assert!(response_str.contains("Connection: keep-alive"));
        assert!(response_str.contains("Keep-Alive: timeout=10"));
        assert!(response_str.contains("Server: HTeaPot/"));
        assert!(second_response_str.contains("Second Request"));
    }
}
