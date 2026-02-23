use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Instant;

use super::BUFFER_SIZE;
use super::KEEP_ALIVE_TTL;
use crate::{HttpRequest, HttpResponse, HttpStatus};
// Internal types used for connection management
use super::request::HttpRequestBuilder;
use super::response::{EmptyHttpResponse, HttpResponseCommon, IterError};

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

pub struct Hteapot {
    port: u16,
    address: String,
    threads: u16,
    banned_ips: Vec<SocketAddr>,
    shutdown_signal: Option<Arc<AtomicBool>>,
    shutdown_hooks: Vec<Arc<dyn Fn() + Send + Sync + 'static>>,
}

#[derive(PartialEq)]
enum Status {
    Read,
    Write,
    Finish,
}

/// Represents the state of a connection's lifecycle.
struct SocketStatus {
    ttl: Instant,
    status: Status,
    response: Box<dyn HttpResponseCommon>,
    request: HttpRequestBuilder,
    index_writed: usize,
}

/// Wraps a TCP stream and its associated state.
struct SocketData {
    stream: TcpStream,
    _addr: SocketAddr,
    status: Option<SocketStatus>,
}

impl Hteapot {
    pub fn set_shutdown_signal(&mut self, signal: Arc<AtomicBool>) {
        self.shutdown_signal = Some(signal);
    }

    pub fn get_shutdown_signal(&self) -> Option<Arc<AtomicBool>> {
        self.shutdown_signal.clone()
    }

    pub fn add_banned_ip(&mut self, ip: SocketAddr) {
        self.banned_ips.push(ip);
    }

    pub fn add_shutdown_hook<F>(&mut self, hook: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.shutdown_hooks.push(Arc::new(hook));
    }

    pub fn get_addr(&self) -> (String, u16) {
        return (self.address.clone(), self.port);
    }

    // Constructor
    pub fn new(address: &str, port: u16) -> Self {
        Hteapot {
            port,
            address: address.to_string(),
            threads: 1,
            banned_ips: Vec::new(),
            shutdown_signal: None,
            shutdown_hooks: Vec::new(),
        }
    }

    pub fn new_threaded(address: &str, port: u16, threads: u16) -> Self {
        Hteapot {
            port,
            address: address.to_string(),
            threads: if threads == 0 { 1 } else { threads },
            banned_ips: Vec::new(),
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

        let pool: Arc<(Mutex<VecDeque<(TcpStream, SocketAddr)>>, Condvar)> =
            Arc::new((Mutex::new(VecDeque::new()), Condvar::new()));
        let priority_list: Arc<Mutex<Vec<usize>>> =
            Arc::new(Mutex::new(vec![0; self.threads as usize]));
        let arc_action = Arc::new(action);

        // Clone shutdown_signal and share the shutdown_hooks via Arc
        let shutdown_signal = self.shutdown_signal.clone();
        let shutdown_hooks = Arc::new(self.shutdown_hooks.clone());

        for _thread_index in 0..self.threads {
            let pool_clone = pool.clone();
            let action_clone = arc_action.clone();
            let _priority_list_clone = priority_list.clone();
            let shutdown_signal_clone = shutdown_signal.clone();

            thread::spawn(move || {
                let mut streams_to_handle = Vec::new();
                loop {
                    {
                        let (lock, cvar) = &*pool_clone;
                        let mut pool = lock.lock().expect("Error locking pool");

                        if streams_to_handle.is_empty() {
                            // Store the returned guard back into pool
                            pool = if let Some(signal) = &shutdown_signal_clone {
                                if !signal.load(Ordering::SeqCst) {
                                    break;
                                }
                                cvar.wait_while(pool, |pool| {
                                    pool.is_empty() && signal.load(Ordering::SeqCst)
                                })
                                .expect("Error waiting on cvar")
                            } else {
                                cvar.wait_while(pool, |pool| pool.is_empty())
                                    .expect("Error waiting on cvar")
                            };
                        }

                        while let Some((stream, addr)) = pool.pop_back() {
                            let socket_status = SocketStatus {
                                ttl: Instant::now(),
                                status: Status::Read,
                                response: Box::new(EmptyHttpResponse {}),
                                request: HttpRequestBuilder::new_with_addr(addr),
                                index_writed: 0,
                            };
                            let socket_data = SocketData {
                                _addr: addr,
                                stream,
                                status: Some(socket_status),
                            };
                            streams_to_handle.push(socket_data);
                        }
                    }

                    // {
                    //     let mut priority_list = priority_list_clone
                    //         .lock()
                    //         .expect("Error locking priority list");
                    //     priority_list[thread_index as usize] = streams_to_handle.len();
                    // }

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

            let (stream, addr) = match listener.accept() {
                Ok((stream, addr)) => (stream, addr),
                Err(_) => continue,
            };
            if self.banned_ips.contains(&addr) {
                let _ = stream.shutdown(Shutdown::Both);
                continue;
            }

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
                pool.push_front((stream, addr));
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
        if Instant::now().duration_since(status.ttl) > KEEP_ALIVE_TTL
            && status.status != Status::Write
        {
            let _ = socket_data.stream.shutdown(Shutdown::Both);
            return None;
        }

        match status.status {
            Status::Read => {
                while !status.request.done() {
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
                                    HttpResponse::new(HttpStatus::BadRequest, error_msg, None)
                                        .to_bytes();
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
                    .get("connection")
                    .map(|v| v.to_lowercase() == "keep-alive")
                    .unwrap_or(false);

                let mut response = action(request);
                if keep_alive {
                    response
                        .base()
                        .headers
                        .entry("connection")
                        .or_insert("keep-alive".to_string());
                    response.base().headers.insert(
                        "Keep-Alive",
                        &format!("timeout={}", KEEP_ALIVE_TTL.as_secs()),
                    );
                } else {
                    response.base().headers.insert("Connection", "close");
                }
                status.status = Status::Write;
                status.response = response;
                status.response.set_stream(&socket_data.stream);
            }
            Status::Write => {
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
                status.status = Status::Finish;
                let request = status.request.get()?;
                let keep_alive = request
                    .headers
                    .get("connection")
                    .map(|v| v.to_lowercase() == "keep-alive")
                    .unwrap_or(false);
                if keep_alive {
                    status.status = Status::Read;
                    status.index_writed = 0;
                    status.request = HttpRequestBuilder::new();
                    return Some(());
                } else {
                    let _ = socket_data.stream.shutdown(Shutdown::Both);
                    return None;
                }
            }
            Status::Finish => {
                return None;
            }
        };
        Some(())

        // If the request is not yet complete, read data from the stream into a buffer.
        // This ensures that the server can handle partial or chunked requests.

        // Seting the stream in case is needed for the response, (example: streaming)
        // Write the response to the client in chunks
    }
}
