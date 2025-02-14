// Written by Alberto Ruiz 2024-03-08
// This is the HTTP server module, it will handle the requests and responses
// Also provide utilities to parse the requests and build the responses

extern crate native_tls;

use self::native_tls::TlsStream;

use self::native_tls::{Identity, TlsAcceptor};

pub mod brew;
mod methods;
mod request;
mod response;
mod status;

pub use self::methods::HttpMethod;
pub use self::request::HttpRequest;
pub use self::response::HttpResponse;
pub use self::status::HttpStatus;

use std::collections::{HashMap, VecDeque};
use std::error::Error;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, ErrorKind, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::ops::Deref;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

const VERSION: &str = env!("CARGO_PKG_VERSION");

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

pub struct SslConfig {
    cert: String,
    password: String,
}

pub struct Hteapot {
    port: u16,
    address: String,
    threads: u16,
    ssl_config: Option<SslConfig>,
}

#[derive(Clone, Debug)]
struct SocketStatus {
    // TODO: write proper ttl
    reading: bool,
    data_readed: Vec<u8>,
    data_write: Vec<u8>,
    index_writed: usize,
}

enum Stream {
    Plain(TcpStream),
    Tls(TlsStream<TcpStream>),
}

struct SocketData {
    stream: Stream,
    status: Option<SocketStatus>,
}

impl Hteapot {
    // Constructor
    pub fn new(address: &str, port: u16) -> Self {
        Hteapot {
            port,
            address: address.to_string(),
            threads: 1,
            ssl_config: None, //cache: HashMap::new(),
        }
    }

    pub fn new_threaded(address: &str, port: u16, threads: u16) -> Self {
        Hteapot {
            port,
            address: address.to_string(),
            threads: if threads == 0 { 1 } else { threads },
            ssl_config: None, //cache: HashMap::new(),
                              //cache: HashMap::new(),
        }
    }

    pub fn set_ssl(&mut self, cert: &str, pass: &str) -> &mut Self {
        self.ssl_config = Some(SslConfig {
            cert: cert.to_string(),
            password: pass.to_string(),
        });
        return self;
    }

    // Start the server
    pub fn listen(&self, action: impl Fn(HttpRequest) -> HttpResponse + Send + Sync + 'static) {
        let addr = format!("{}:{}", self.address, self.port);
        let mut acceptor: Option<TlsAcceptor> = None;
        if self.ssl_config.is_some() {
            acceptor = {
                let mut file = File::open(&self.ssl_config.as_ref().unwrap().cert).unwrap();
                let mut cert_data = Vec::new();
                file.read_to_end(&mut cert_data).unwrap();
                let identity =
                    Identity::from_pkcs12(&cert_data, &self.ssl_config.as_ref().unwrap().password)
                        .unwrap();

                let acceptor = TlsAcceptor::new(identity).unwrap();
                Some(acceptor)
            };
        }

        let listener = TcpListener::bind(addr);
        let listener = match listener {
            Ok(listener) => listener,
            Err(e) => {
                eprintln!("Error L: {}", e);
                return;
            }
        };

        let pool: Arc<(Mutex<VecDeque<Stream>>, Condvar)> =
            Arc::new((Mutex::new(VecDeque::new()), Condvar::new()));

        //let statusPool = Arc::new(Mutex::new(HashMap::<String, socketStatus>::new()));
        let priority_list: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));
        let arc_action = Arc::new(action);
        for _tn in 0..self.threads {
            let _tn = _tn as usize;
            let pool_clone = pool.clone();
            let action_clone = arc_action.clone();
            let pl_clone = priority_list.clone();
            {
                let mut pl_lock = pl_clone.lock().expect("Error locking prority list");
                pl_lock.push(0);
            }
            thread::spawn(move || {
                let mut streams_to_handle = Vec::new();
                loop {
                    {
                        let (lock, cvar) = &*pool_clone;
                        let mut pool = lock.lock().expect("Error locking pool");
                        let pl_copy;
                        {
                            let pl_lock = pl_clone.lock().expect("Error locking prority list");
                            pl_copy = pl_lock.clone();
                        }

                        if streams_to_handle.is_empty() {
                            pool = cvar
                                .wait_while(pool, |pool| pool.is_empty())
                                .expect("Error waiting on cvar");
                        } else if pl_copy.len() != 1
                            && streams_to_handle.len() < 10
                            && pl_copy
                                .iter()
                                .find(|&&v| streams_to_handle.len() > v)
                                .is_none()
                        {
                            (pool, _) = cvar
                                .wait_timeout_while(pool, Duration::from_millis(500), |pool| {
                                    pool.is_empty()
                                })
                                .expect("Error waiting on cvar");
                        }

                        if !pool.is_empty() {
                            let socket_status = SocketStatus {
                                reading: true,
                                data_readed: vec![],
                                data_write: vec![],
                                index_writed: 0,
                            };
                            let socket_data = SocketData {
                                stream: pool.pop_back().unwrap(),
                                status: Some(socket_status),
                            };
                            streams_to_handle.push(socket_data);

                            {
                                let mut pl_lock =
                                    pl_clone.lock().expect("Errpr locking prority list");
                                pl_lock[_tn] = streams_to_handle.len();
                            }
                        }
                    }

                    for stream_data in streams_to_handle.iter_mut() {
                        if stream_data.status.is_none() {
                            continue;
                        }
                        let r = Hteapot::handle_client(
                            &mut stream_data.stream,
                            stream_data.status.as_mut().unwrap().clone(),
                            &action_clone,
                        );
                        stream_data.status = r;
                    }
                    streams_to_handle.retain(|s| s.status.is_some());
                    {
                        let mut pl_lock = pl_clone.lock().expect("Errpr locking prority list");
                        pl_lock[_tn] = streams_to_handle.len();
                    }
                }
            });
        }

        let pool_clone = pool.clone();
        loop {
            let stream = listener.accept();

            if stream.is_err() {
                continue;
            }
            let (stream, _) = stream.unwrap();

            {
                let (lock, cvar) = &*pool_clone;
                let mut pool = lock.lock().expect("Error locking pool");
                if acceptor.as_ref().is_none() {
                    stream
                        .set_nonblocking(true)
                        .expect("Error seting non blocking");
                    stream.set_nodelay(true).expect("Error seting no delay");
                    pool.push_front(Stream::Plain(stream));
                } else {
                    let acceptor = acceptor.as_ref().unwrap();
                    loop {
                        let _stream = stream.try_clone().expect("msg");
                        let tls_stream = acceptor.accept(_stream);
                        match tls_stream {
                            Ok(tls_stream) => {
                                pool.push_front(Stream::Tls(tls_stream));
                                break;
                            }
                            Err(e) => {
                                println!("{:?}", e);
                                let e = e.source();
                                if e.is_none() {
                                    continue;
                                }
                                pool.push_front(Stream::Plain(stream.try_clone().expect("msg")));
                                break;
                            }
                        }
                    }
                }
                cvar.notify_one();
            }
            // Notify one waiting thread
        }
    }

    // Parse the request
    pub fn request_parser(request: String) -> Result<HttpRequest, String> {
        let mut lines = request.lines();
        let first_line = lines.next();
        if first_line.is_none() {
            println!("{}", request);
            return Err("Invalid request".to_string());
        }
        let first_line = first_line.unwrap();
        let mut words = first_line.split_whitespace();
        let method = words.next();
        if method.is_none() {
            return Err("Invalid method".to_string());
        }
        let method = method.unwrap();
        let path = words.next();
        if path.is_none() {
            return Err("Invalid path".to_string());
        }
        let mut path = path.unwrap().to_string();
        let mut headers: HashMap<String, String> = HashMap::new();
        loop {
            let line = lines.next();
            if line.is_none() {
                break;
            }
            let line = line.unwrap();
            if line.is_empty() {
                break;
            }
            let mut parts = line.split(": ");
            let key = parts.next().unwrap().to_string();
            let value = parts.next().unwrap();
            headers.insert(key, value.to_string());
        }
        let body = lines
            .collect::<Vec<&str>>()
            .join("")
            .trim()
            .trim_end_matches(char::from(0))
            .to_string();
        let mut args: HashMap<String, String> = HashMap::new();
        //remove http or https from the path
        if path.starts_with("http://") {
            path = path.trim_start_matches("http://").to_string();
        } else if path.starts_with("https://") {
            path = path.trim_start_matches("https://").to_string();
        }
        //remove the host name if present
        if !path.starts_with("/") {
            //remove all the characters until the first /
            let mut parts = path.split("/");
            parts.next();
            path = parts.collect::<Vec<&str>>().join("/");
            //add / to beggining
            path = format!("/{}", path);
        }

        if path.contains('?') {
            let _path = path.clone();
            let mut parts = _path.split('?');
            path = parts.next().unwrap().to_string();
            let query = parts.next().unwrap();
            let query_parts: Vec<&str> = query.split('&').collect();
            for part in query_parts {
                let mut parts = part.split('=');
                let key = parts.next().unwrap().to_string();
                let value = parts.next().unwrap_or("").to_string().replace("%22", "\"");
                args.insert(key, value);
            }
        }

        Ok(HttpRequest {
            method: HttpMethod::from_str(method),
            path: path.to_string(),
            args,
            headers,
            body: body.trim_end().to_string(),
        })
    }

    // Handle the client when a request is received
    fn handle_client(
        stream: &mut Stream,
        socket_status: SocketStatus,
        action: &Arc<impl Fn(HttpRequest) -> HttpResponse + Send + Sync + 'static>,
    ) -> Option<SocketStatus> {
        //let mut reader = BufReader::new(stream);
        //let mut writer = BufWriter::new(stream);
        println!("handling...");
        let mut socket_status = socket_status.clone();
        if socket_status.reading {
            loop {
                let mut buffer = [0; 1024];
                let reader_result = match stream {
                    Stream::Plain(stream) => stream.read(&mut buffer),
                    Stream::Tls(stream) => stream.read(&mut buffer),
                };
                match reader_result {
                    Err(e) => match e.kind() {
                        io::ErrorKind::WouldBlock => {
                            return Some(socket_status);
                        }
                        io::ErrorKind::ConnectionReset => {
                            return None;
                        }
                        _ => {
                            println!("R Error{:?}", e);
                            return None;
                        }
                    },
                    Ok(m) => {
                        if m == 0 {
                            return None;
                        }
                    }
                };
                socket_status.data_readed.append(&mut buffer.to_vec());
                //socket_status
                if buffer[0] == 0 {
                    break;
                };
                if *buffer.last().unwrap() == 0 {
                    break;
                }
            }
            socket_status.reading = false;
        }

        let request_string = String::from_utf8(socket_status.data_readed.clone());
        let request_string = if request_string.is_err() {
            //This proablly means the request is a https so for the moment GTFO
            return None;
        } else {
            request_string.unwrap()
        };
        // let request_string = "GET / HTTP/1.1\r\nHost: example.com\r\nConnection: close\r\n\r\n".to_string();
        let request = Self::request_parser(request_string);
        if request.is_err() {
            eprintln!("Request parse error {:?}", request.err().unwrap());
            return None;
        }
        let request = request.unwrap();
        let keep_alive = match request.headers.get("Connection") {
            Some(ch) => ch == "keep-alive",
            None => false,
        };
        if socket_status.data_write.len() == 0 {
            let mut response = action(request);
            if !response.headers.contains_key("Conection") && keep_alive {
                response
                    .headers
                    .insert("Connection".to_string(), "keep_alive".to_string());
            } else {
                response
                    .headers
                    .insert("Connection".to_string(), "close".to_string());
            }
            socket_status.data_write = response.to_bytes();
        }
        for n in socket_status.index_writed..socket_status.data_write.len() {
            //let r = writer.write();
            let r = match stream {
                Stream::Plain(stream) => stream.write(&[socket_status.data_write[n]]),
                Stream::Tls(stream) => stream.write(&[socket_status.data_write[n]]),
            };
            if r.is_err() {
                let error = r.err().unwrap();
                if error.kind() == io::ErrorKind::WouldBlock {
                    return Some(socket_status);
                } else {
                    eprintln!("W error: {:?}", error);
                    return None;
                }
            }
            socket_status.index_writed += r.unwrap();
        }

        let r = match stream {
            Stream::Plain(stream) => stream.flush(),
            Stream::Tls(stream) => stream.flush(),
        };
        if r.is_err() {
            eprintln!("Error2: {}", r.err().unwrap());
            return Some(socket_status);
        }
        if keep_alive {
            socket_status.reading = true;
            socket_status.data_readed = vec![];
            socket_status.data_write = vec![];
            socket_status.index_writed = 0;
            return Some(socket_status);
        } else {
            println!("Done");
            let _ = match stream {
                Stream::Plain(stream) => {
                    stream.shutdown(Shutdown::Both);
                }
                Stream::Tls(stream) => {
                    stream.shutdown();
                }
            };
            None
        }
    }
}

#[cfg(test)]
#[test]
fn test_http_parser() {
    let request =
        "GET / HTTP/1.1\r\nHost: localhost:8080\r\nUser-Agent: curl/7.68.0\r\nAccept: */*\r\n\r\n";
    let parsed_request = Hteapot::request_parser(request.to_string()).unwrap();
    assert_eq!(parsed_request.method, HttpMethod::GET);
    assert_eq!(parsed_request.path, "/");
    assert_eq!(parsed_request.args.len(), 0);
    assert_eq!(parsed_request.headers.len(), 3);
    assert_eq!(parsed_request.body, "");
}

#[test]
fn test_http_response_maker() {
    let response = HttpResponse::new(HttpStatus::IAmATeapot, "Hello, World!", None);
    let response = String::from_utf8(response.to_bytes()).unwrap();
    let expected_response = format!("HTTP/1.1 418 I'm a teapot\r\nContent-Length: 13\r\nServer: HTeaPot/{}\r\n\r\nHello, World!\r\n",VERSION);
    let expected_response_list = expected_response.split("\r\n");
    for item in expected_response_list.into_iter() {
        assert!(response.contains(item));
    }
}
