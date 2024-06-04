// Written by Alberto Ruiz 2024-03-08
// This is the HTTP server module, it will handle the requests and responses
// Also provide utilities to parse the requests and build the responses


use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

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


pub struct HteaPot {
    port: u16,
    address: String,
    // this will store a map from path to their actions
    // path_table: HashMap<HttpMethod, HashMap<String, HashMap<HttpMethod, fn(HttpRequest) -> String>>>,
}

impl HteaPot {

    // Constructor
    pub fn new(address: &str, port: u16) -> Self {
        HteaPot {
            port: port,
            address: address.to_string(),
            // path_table: HashMap::new(),
        }
    }

    // Start the server
    pub fn listen(&self, action: impl Fn(HttpRequest) -> String ){
        let addr = format!("{}:{}", self.address, self.port);
        let listener = TcpListener::bind(addr);
        let listener = match listener {
            Ok(listener) => listener,
            Err(e) => {
                eprintln!("Error: {}", e);
                return;
            }
        };
        for stream in listener.incoming() {
            match stream {
                 Ok(stream) => {
                //     thread::spawn(move || {
                //         HteaPot::handle_client(stream);
                //     });
                    self.handle_client(stream, &action)
   
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
        }
    }


    // Create a response
    pub fn response_maker(status: HttpStatus, content: &str, headers: Option<HashMap<String,String>>) -> String {
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
        let response = format!("HTTP/1.1 {} {}\r\n{}\r\n{}",status as u16, status_text,headers_text ,content);
        response
    }

    // Parse the request
    pub fn request_parser(request: &str) -> HttpRequest {
        let mut lines = request.lines();
        let first_line = lines.next().unwrap();
        let mut words = first_line.split_whitespace();
        let method = words.next().unwrap();
        let mut path = words.next().unwrap().to_string();
        let mut headers: HashMap<String, String> = HashMap::new();
        loop {
            let line = lines.next().unwrap();
            if line.is_empty() {
                break;
            }
            let mut parts = line.split(": ");
            let key = parts.next().unwrap().to_string();
            let value = parts.next().unwrap();
            headers.insert(key, value.to_string());
        }
        let remaining_lines: Vec<&str>  = lines.collect();
        let body = remaining_lines.join("");
        let body = body.trim().trim_end();
        //remove all traling zero bytes
        let body = body.trim_matches(char::from(0));
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

        HttpRequest {
            method: HttpMethod::from_str(method),
            path: path.to_string(),
            args: args,
            headers: headers,
            body: body.trim_end().to_string(),
        }
    }

    // Handle the client when a request is received
    fn handle_client(&self, mut stream: TcpStream , action: impl Fn(HttpRequest) -> String ) {
        let mut buffer = [0; 1024];
        stream.read(&mut buffer).unwrap(); //TODO: handle the error
        let request_buffer = String::from_utf8_lossy(&buffer);
        let request = Self::request_parser(&request_buffer);
        //let response = Self::response_maker(HttpStatus::IAmATeapot, "Hello, World!");
        let response = action(request);
        let r = stream.write(response.as_bytes()); 
        if r.is_err() {
            eprintln!("Error: {}", r.err().unwrap());
        }
        let r = stream.flush();
        if r.is_err() {
            eprintln!("Error: {}", r.err().unwrap());
        }
    }
}


#[cfg(test)]

#[test]
fn test_http_parser() {
    let request = "GET / HTTP/1.1\r\nHost: localhost:8080\r\nUser-Agent: curl/7.68.0\r\nAccept: */*\r\n\r\n";
    let parsed_request = HteaPot::request_parser(request);
    assert_eq!(parsed_request.method, HttpMethod::GET);
    assert_eq!(parsed_request.path, "/");
    assert_eq!(parsed_request.args.len(), 0);
    assert_eq!(parsed_request.headers.len(), 3);
    assert_eq!(parsed_request.body, "");
}

#[test]
fn test_http_response_maker() {
    let response = HteaPot::response_maker(HttpStatus::IAmATeapot, "Hello, World!", None);
    let expected_response = "HTTP/1.1 418 I'm a teapot\r\nContent-Length: 13\r\n\r\nHello, World!";
    assert_eq!(response, expected_response);
}

