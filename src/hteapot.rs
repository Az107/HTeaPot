// Written by Alberto Ruiz 2024-03-08
// This is the HTTP server module, it will handle the requests and responses
// Also provide utilities to parse the requests and build the responses

use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use  s

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
    TRACE,
    CONNECT,
    Other(String),
}

impl HttpMethod {
    pub fn from_str(method: &str) -> HttpMethod {
        match method {
            "GET" => HttpMethod::GET,
            "POST" => HttpMethod::POST,
            "PUT" => HttpMethod::PUT,
            "DELETE" => HttpMethod::DELETE,
            "PATCH" => HttpMethod::PATCH,
            "HEAD" => HttpMethod::HEAD,
            "OPTIONS" => HttpMethod::OPTIONS,
            "TRACE" => HttpMethod::TRACE,
            "CONNECT" => HttpMethod::CONNECT,
            _ => Self::Other(method.to_string()),
        }
    }
    pub fn to_str(&self) -> &str {
        match self {
            HttpMethod::GET => "GET",
            HttpMethod::POST => "POST",
            HttpMethod::PUT => "PUT",
            HttpMethod::DELETE => "DELETE",
            HttpMethod::PATCH => "PATCH",
            HttpMethod::HEAD => "HEAD",
            HttpMethod::OPTIONS => "OPTIONS",
            HttpMethod::TRACE => "TRACE",
            HttpMethod::CONNECT => "CONNECT",
            HttpMethod::Other(method) => method.as_str(),
        }
    }
}

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

#[derive(Clone, Copy)]
pub enum Protocol {
    HTTP,
    HTTPS,
}

#[derive(Clone, Copy)]
pub enum HttpStatus {
    OK = 200,
    Created = 201,
    Accepted = 202,
    NoContent = 204,
    MovedPermanently = 301,
    MovedTemporarily = 302,
    NotModified = 304,
    BadRequest = 400,
    Unauthorized = 401,
    Forbidden = 403,
    NotFound = 404,
    IAmATeapot = 418,
    InternalServerError = 500,
    NotImplemented = 501,
    BadGateway = 502,
    ServiceUnavailable = 503,
}

impl HttpStatus {
    pub fn from_u16(status: u16) -> HttpStatus {
        match status {
            200 => HttpStatus::OK,
            201 => HttpStatus::Created,
            202 => HttpStatus::Accepted,
            204 => HttpStatus::NoContent,
            301 => HttpStatus::MovedPermanently,
            302 => HttpStatus::MovedTemporarily,
            304 => HttpStatus::NotModified,
            400 => HttpStatus::BadRequest,
            401 => HttpStatus::Unauthorized,
            403 => HttpStatus::Forbidden,
            404 => HttpStatus::NotFound,
            418 => HttpStatus::IAmATeapot,
            500 => HttpStatus::InternalServerError,
            501 => HttpStatus::NotImplemented,
            502 => HttpStatus::BadGateway,
            503 => HttpStatus::ServiceUnavailable,
            _ => panic!("Invalid HTTP status"),
        }
    }

    fn to_string(&self) -> &str {
        match self {
            HttpStatus::OK => "OK",
            HttpStatus::Created => "Created",
            HttpStatus::Accepted => "Accepted",
            HttpStatus::NoContent => "No Content",
            HttpStatus::MovedPermanently => "Moved Permanently",
            HttpStatus::MovedTemporarily => "Moved Temporarily",
            HttpStatus::NotModified => "Not Modified",
            HttpStatus::BadRequest => "Bad Request",
            HttpStatus::Unauthorized => "Unauthorized",
            HttpStatus::Forbidden => "Forbidden",
            HttpStatus::NotFound => "Not Found",
            HttpStatus::IAmATeapot => "I'm a teapot",
            HttpStatus::InternalServerError => "Internal Server Error",
            HttpStatus::NotImplemented => "Not Implemented",
            HttpStatus::BadGateway => "Bad Gateway",
            HttpStatus::ServiceUnavailable => "Service Unavailable",
        }
    }
}

pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub args: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: String,
}

pub struct HttpResponse {
    pub status: HttpStatus,
    pub headers: HashMap<String, String>,
    pub content: Vec<u8>,
    raw: Option<Vec<u8>>,
    is_raw: bool,
}

impl HttpResponse {
    pub fn new<B: AsRef<[u8]>>(
        status: HttpStatus,
        content: B,
        headers: Option<HashMap<String, String>>,
    ) -> Self {
        let mut headers = headers.unwrap_or(HashMap::new());
        let content = content.as_ref();
        headers.insert("Content-Length".to_string(), content.len().to_string());
        headers.insert(
            "Server".to_string(),
            format!("HTeaPot/{}", VERSION).to_string(),
        );
        HttpResponse {
            status,
            headers,
            content: content.to_owned(),
            raw: None,
            is_raw: false,
        }
    }

    pub fn new_raw(raw: Vec<u8>) -> Self {
        HttpResponse {
            status: HttpStatus::IAmATeapot,
            headers: HashMap::new(),
            content: vec![],
            raw: Some(raw),
            is_raw: true,
        }
    }

    pub fn is_raw(&self) -> bool {
        self.is_raw
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        if self.is_raw() {
            return self.raw.clone().unwrap();
        }
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
        response.append(&mut self.content.clone());
        response.push(0x0D); // Carriage Return
        response.push(0x0A); // Line Feed
        response
    }
}

pub struct Hteapot {
    port: u16,
    address: String,
    threads: u16,
    //cache: HashMap<String,String>,
    //pool: Arc<(Mutex<Vec<TcpStream>>, Condvar)>,
}

#[derive(Clone, Debug)]
struct SocketStatus {
    // TODO: write proper ttl
    reading: bool,
    data_readed: Vec<u8>,
    data_write: Vec<u8>,
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
            //cache: HashMap::new(),
        }
    }

    pub fn new_threaded(address: &str, port: u16, threads: u16) -> Self {
        Hteapot {
            port,
            address: address.to_string(),
            threads: if threads == 0 { 1 } else { threads },
            //cache: HashMap::new(),
        }
    }

    // Start the server
    pub fn listen(&self, action: impl Fn(HttpRequest) -> HttpResponse + Send + Sync + 'static) {
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
        //let statusPool = Arc::new(Mutex::new(HashMap::<String, socketStatus>::new()));
        let arc_action = Arc::new(action);
        for _tn in 0..self.threads {
            let pool_clone = pool.clone();
            let action_clone = arc_action.clone();
            thread::spawn(move || {
                let mut streams_to_handle = Vec::new();
                loop {
                    {
                        let (lock, cvar) = &*pool_clone;
                        let mut pool = lock.lock().expect("Error locking pool");
                        if streams_to_handle.len() < 10 {
                            pool = cvar
                                .wait_while(pool, |pool| pool.is_empty())
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
                        }
                    }

                    for stream_data in streams_to_handle.iter_mut() {
                        if stream_data.status.is_none() {
                            continue;
                        }
                        let r = Hteapot::handle_client(
                            &stream_data.stream,
                            stream_data.status.as_mut().unwrap().clone(),
                            &action_clone,
                        );
                        stream_data.status = r;
                    }
                    streams_to_handle.retain(|s| s.status.is_some());
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
            stream
                .set_nonblocking(true)
                .expect("Error seting non blocking");
            stream.set_nodelay(true).expect("Error seting no delay");
            {
                let (lock, cvar) = &*pool_clone;
                let mut pool = lock.lock().expect("Error locking pool");

                pool.push_front(stream);
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
            args: args,
            headers: headers,
            body: body.trim_end().to_string(),
        })
    }

    // Handle the client when a request is received
    fn handle_client(
        stream: &TcpStream,
        socket_status: SocketStatus,
        action: &Arc<impl Fn(HttpRequest) -> HttpResponse + Send + Sync + 'static>,
    ) -> Option<SocketStatus> {
        let mut reader = BufReader::new(stream);
        let mut writer = BufWriter::new(stream);
        let mut socket_status = socket_status.clone();
        if socket_status.reading {
            loop {
                let mut buffer = [0; 1024];
                match reader.read(&mut buffer) {
                    Err(e) => match e.kind() {
                        io::ErrorKind::WouldBlock => {
                            return Some(socket_status);
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

        let request_string = String::from_utf8(socket_status.data_readed.clone()).unwrap();
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
            if keep_alive {
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
            let r = writer.write(&[socket_status.data_write[n]]);
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

        let r = writer.flush();
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
            let _ = stream.shutdown(Shutdown::Both);
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
