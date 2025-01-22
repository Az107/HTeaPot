// Written by Alberto Ruiz 2024-04-08
// This is the HTTP client module, it will handle the requests and responses

use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use super::methods::HttpMethod;
use super::request::HttpRequest;
use super::response::HttpResponse;
use super::status::HttpStatus;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

impl HttpRequest {
    pub fn new(method: HttpMethod, path: &str) -> HttpRequest {
        let path = path.to_string();
        HttpRequest {
            method,
            path,
            args: HashMap::new(),
            headers: HashMap::new(),
            body: String::new(),
        }
    }

    pub fn arg(&mut self, key: &str, value: &str) -> &mut HttpRequest {
        self.args.insert(key.to_string(), value.to_string());
        return self;
    }

    pub fn header(&mut self, key: &str, value: &str) -> &mut HttpRequest {
        self.headers.insert(key.to_string(), value.to_string());
        return self;
    }

    pub fn body(&mut self, body: String) -> &mut HttpRequest {
        self.body = body;
        return self;
    }

    pub fn to_string(&self) -> String {
        let path = if self.args.is_empty() {
            self.path.clone()
        } else {
            let mut path = self.path.clone();
            path.push('?');
            for (k, v) in self.args.iter() {
                path.push_str(format!("{}={}&", k, v).as_str());
            }
            if path.ends_with('&') {
                path.pop();
            }
            path
        };
        let path = if path.is_empty() {
            "/".to_string()
        } else {
            path
        };
        let mut result: String = format!("{} {} HTTP/1.1\r\n", self.method.to_str(), path);
        for (k, v) in self.headers.iter() {
            result.push_str(format!("{}: {}\r\n", k, v).as_str());
        }
        if !self.body.is_empty() {
            result.push_str(self.body.as_str());
        }
        result.push_str("\r\n\r\n");
        result
    }

    pub fn brew(&self, addr: &str) -> Result<HttpResponse, &'static str> {
        let mut addr = addr.to_string();
        if addr.starts_with("http://") {
            addr = addr.strip_prefix("http://").unwrap().to_string();
        } else if addr.starts_with("https://") {
            return Err("Not implemented yet");
        }
        if !addr.contains(':') {
            let _addr = format!("{}:80", addr.clone());
            addr = _addr
        }
        let addr: Vec<_> = addr
            .to_socket_addrs()
            .expect("Unable to resolve domain")
            .collect();
        let addr = addr.first().expect("Error parsing address");
        let stream = TcpStream::connect_timeout(addr, Duration::from_secs(5));
        if stream.is_err() {
            return Err("Error connecting to server");
        }

        let mut stream = stream.unwrap();
        let _ = stream.write(self.to_string().as_bytes());
        let _ = stream.flush();
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let mut raw: Vec<u8> = Vec::new();
        let _ = stream.read_to_end(&mut raw);

        Ok(HttpResponse::new_raw(raw))
    }
}

pub fn brew(direction: &str, request: HttpRequest) -> Result<HttpResponse, &'static str> {
    return request.brew(direction);
}

pub fn brew_url(url: &str) -> Result<HttpResponse, &'static str> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_request_new() {
        let request = HttpRequest::new(HttpMethod::GET, "/example");
        assert_eq!(request.method, HttpMethod::GET);
        assert_eq!(request.path, "/example");
        assert!(request.args.is_empty());
        assert!(request.headers.is_empty());
        assert_eq!(request.body, "");
    }

    #[test]
    fn test_http_request_arg() {
        let mut request = HttpRequest::new(HttpMethod::POST, "/submit");
        request.arg("key", "value");
        assert_eq!(request.args.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_http_request_header() {
        let mut request = HttpRequest::new(HttpMethod::GET, "/data");
        request.header("Content-Type", "application/json");
        assert_eq!(
            request.headers.get("Content-Type"),
            Some(&"application/json".to_string())
        );
    }

    #[test]
    fn test_http_request_body() {
        let mut request = HttpRequest::new(HttpMethod::POST, "/upload");
        request.body("Test body content".to_string());
        assert_eq!(request.body, "Test body content");
    }

    #[test]
    fn test_http_request_to_string() {
        let mut request = HttpRequest::new(HttpMethod::POST, "/resource");
        request
            .header("Content-Type", "application/json")
            .body("{\"data\":\"test\"}".to_string());

        let request_string = request.to_string();
        assert!(request_string.contains("POST /resource HTTP/1.1"));
        assert!(request_string.contains("Content-Type: application/json"));
        assert!(request_string.contains("{\"data\":\"test\"}"));
    }

    #[test]
    fn test_http_request_to_string_with_args() {
        let mut request = HttpRequest::new(HttpMethod::POST, "/resource");
        let _ = request
            .header("Content-Type", "application/json")
            .arg("key", "value")
            .body("{\"data\":\"test\"}".to_string())
            .brew("localhost:8080");

        let request_string = request.to_string();
        assert!(request_string.contains("POST /resource?key=value HTTP/1.1"));
        assert!(request_string.contains("Content-Type: application/json"));
        assert!(request_string.contains("{\"data\":\"test\"}"));
    }

    #[test]
    fn test_http_request() {
        let r = HttpRequest::new(HttpMethod::GET, "/").brew("example.org:80");
        assert!(r.is_ok());
    }

    #[test]
    fn test_http_request_time_out() {
        let r = HttpRequest::new(HttpMethod::GET, "/").brew("example.org:8080");
        assert!(r.is_err());
    }
}
