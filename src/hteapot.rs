// Written by Alberto Ruiz 2024-03-08
// This is the HTTP server module, it will handle the requests and responses
// Also provide utilities to parse the requests and build the responses

use std::collections::HashMap;
use std::hash::Hash;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::{str, thread};
use std::sync::{Arc, Mutex, Condvar};


#[derive(Debug)]
#[derive(PartialEq)]
#[derive(Eq)]
#[derive(Hash)]
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
            _ => panic!("Invalid HTTP method"),
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


pub struct Hteapot {
    port: u16,
    address: String,
    //cache: HashMap<String,String>,
    pool: Arc<(Mutex<Vec<TcpStream>>, Condvar)>,

}

impl Hteapot {

    // Constructor
    pub fn new(address: &str, port: u16) -> Self {
        Hteapot {
            port: port,
            address: address.to_string(),
            //cache: HashMap::new(),
            pool: Arc::new((Mutex::new(Vec::new()), Condvar::new())),

        }
    }

    // Start the server
    pub fn listen(&mut self, action: impl Fn(HttpRequest) -> Vec<u8> + Send + Sync + 'static  ){
        let addr = format!("{}:{}", self.address, self.port);
        let listener = TcpListener::bind(addr);
        let listener = match listener {
            Ok(listener) => listener,
            Err(e) => {
                eprintln!("Error: {}", e);
                return;
            }
        };
        let arc_action = Arc::new(action);
        listener.set_nonblocking(false).expect("set_nonblocking call failed");
        let pool_clone = self.pool.clone();
        let greeter_loop = thread::spawn(move || {
            for stream in listener.incoming() {
                if stream.is_err() {
                    println!("error stream! {:?}",stream.err());
                    continue;
                }
                let stream = stream.unwrap();
                let (lock, cvar) = &*pool_clone;
                stream.set_nodelay(true).expect("Error set nodelay to stream");
                println!("waiting");
                let mut pool = lock.lock().expect("Error locking pool");
                println!("locked!!");

                pool.push(stream);
                cvar.notify_one();  // Notify one waiting thread
            }
        });
        let pool_clone = self.pool.clone();
        thread::spawn(move || {
            let mut streams_to_handle = Vec::new();
            loop {
                    {
                        if streams_to_handle.is_empty() {
                            let (lock, cvar) = &*pool_clone;
                            let mut pool = lock.lock().expect("Error locking pool");
        
                            while pool.is_empty(){
                                pool = cvar.wait(pool).expect("Error waiting on cvar");
                            }
        
                            // Movemos los streams fuera del mutex
                            streams_to_handle.append(&mut *pool);
                        }
                    }
                    streams_to_handle.retain(|stream| {
                        let action_clone = arc_action.clone();
                        Hteapot::handle_client(stream, move |request| {
                                    action_clone(request)
                        })
                    });
            }
        });

        greeter_loop.join().expect("Erroing joining listener loop");
    }


    // Create a response
    pub fn response_maker<B: AsRef<[u8]>>(status: HttpStatus, content: B, headers: Option<HashMap<String,String>>) -> Vec<u8> {
        let content = content.as_ref();
        let status_text = status.to_string();
        let mut headers_text = String::new();
        let mut headers = if headers.is_some() {
            headers.unwrap()
        } else {
            HashMap::new()
        };
        headers.insert("Content-Length".to_string(), content.len().to_string());
        for (key, value) in headers.iter() {
            headers_text.push_str(&format!("{}: {}\r\n", key, value));
        }
        let response_header = format!("HTTP/1.1 {} {}\r\n{}\r\n",status as u16, status_text,headers_text);
        let mut response = Vec::new();
        response.extend_from_slice(response_header.as_bytes());
        response.extend_from_slice(content);
        response
    }

    // Parse the request
    pub fn request_parser(request: String) -> Result<HttpRequest, String> {
        let mut lines = request.lines();
        let first_line = lines.next();
        if first_line.is_none() {
            return Err("Error parsng request".to_string());
        }
        let first_line = first_line.unwrap();
        let mut words = first_line.split_whitespace();
        let method = words.next();
        if method.is_none() {
            return Err("Error parsng request".to_string());
        }
        let method = method.unwrap();
        let path = words.next();
        if path.is_none() {
            return Err("Error parsng request".to_string());
        }
        let mut path = path.unwrap().to_string();
        let mut headers: HashMap<String, String> = HashMap::new();
        loop {
            let line = lines.next();
            if line.is_none() {break;}
            let line = line.unwrap();
            if line.is_empty() {
                break;
            }
            let mut parts = line.split(": ");
            let key = parts.next().unwrap().to_string();
            let value = parts.next().unwrap();
            headers.insert(key, value.to_string());
        }
        let body = lines.collect::<Vec<&str>>().join("").trim().trim_end_matches(char::from(0)).to_string();
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
    fn handle_client(stream: &TcpStream , action: impl Fn(HttpRequest) -> Vec<u8> + Send + Sync + 'static  ) -> bool{
        let mut reader = BufReader::new(stream);
        let mut writer = BufWriter::new(stream);
        let mut request_buffer = Vec::new();
        loop {
            let mut buffer = [0; 1024];
            match reader.read(&mut buffer) {
                Err(e) => {
                    match e.kind() {
                        io::ErrorKind::WouldBlock => {
                            return true;
                        },
                        _ => {
                            return false;
                        },
                    }
                },
                Ok(m) => {
                    if m == 0 {
                        return false;
                    }
                },
            };
            request_buffer.append(&mut buffer.to_vec());
            if buffer[0] == 0 {break};
            if *buffer.last().unwrap() == 0 {break;}
        }

        let request_string =  String::from_utf8(request_buffer).unwrap();
        let request = Self::request_parser(request_string);
        if request.is_err() {
            eprintln!("{}", request.err().unwrap());
            return false;
        }
        let request = request.unwrap();
        
        let response = action(request);
        let r = writer.write_all(&response);
        if r.is_err() {
            eprintln!("Error: {}", r.err().unwrap());
        }
        let r = writer.flush();
        if r.is_err() {
            eprintln!("Error: {}", r.err().unwrap());
        }

        let r = reader.read(&mut [0; 1]); 
        r.is_err()
    }
}


#[cfg(test)]

#[test]
fn test_http_parser() {
    let request = "GET / HTTP/1.1\r\nHost: localhost:8080\r\nUser-Agent: curl/7.68.0\r\nAccept: */*\r\n\r\n";
    let parsed_request = Hteapot::request_parser(request.to_string()).unwrap();
    assert_eq!(parsed_request.method, HttpMethod::GET);
    assert_eq!(parsed_request.path, "/");
    assert_eq!(parsed_request.args.len(), 0);
    assert_eq!(parsed_request.headers.len(), 3);
    assert_eq!(parsed_request.body, "");
}

#[test]
fn test_http_response_maker() {
    let response = Hteapot::response_maker(HttpStatus::IAmATeapot, "Hello, World!", None);
    let response = String::from_utf8(response).unwrap();
    let expected_response = "HTTP/1.1 418 I'm a teapot\r\nContent-Length: 13\r\n\r\nHello, World!";
    assert_eq!(response, expected_response);
}

