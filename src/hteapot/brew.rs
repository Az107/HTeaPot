// Written by Alberto Ruiz 2024-04-08
//
// This module provides basic HTTP client functionality. It defines
// methods to compose and send HTTP requests and parse the resulting
// responses using a `TcpStream`.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use super::request::HttpRequest;
use super::response::HttpResponse;
// use super::status::HttpStatus;
// use std::net::{IpAddr, Ipv4Addr, SocketAddr};

impl HttpRequest {
    /// Adds a query argument to the HTTP request.
    pub fn arg(&mut self, key: &str, value: &str) -> &mut HttpRequest {
        self.args.insert(key.to_string(), value.to_string());
        self
    }

    /// Adds a header to the HTTP request.
    pub fn header(&mut self, key: &str, value: &str) -> &mut HttpRequest {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    /// Converts the request into a raw HTTP/1.1-compliant string.
    ///
    /// This includes method, path with optional query args, headers, and optional body.
    pub fn to_string(&self) -> String {
        // Add query parameters to the path if needed
        let path = if self.args.is_empty() {
            self.path.clone()
        } else {
            let mut path = format!("{}?", self.path);
            for (k, v) in &self.args {
                path.push_str(&format!("{}={}&", k, v));
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

        let mut result = format!("{} {} HTTP/1.1\r\n", self.method.to_str(), path);
        for (k, v) in &self.headers {
            result.push_str(&format!("{}: {}\r\n", k, v));
        }

        result.push_str(&self.text().unwrap_or_default());
        result.push_str("\r\n\r\n");

        result
    }

    /// Sends the request to a remote server and returns a parsed response.
    ///
    /// Supports only `http://` (not `https://`). Attempts to resolve the domain
    /// and open a TCP connection. Times out after 5 seconds.
    pub fn brew(&self, addr: &str) -> Result<Box<HttpResponse>, &'static str> {
        let mut addr = addr.to_string();

        // Strip protocol prefix
        if let Some(stripped) = addr.strip_prefix("http://") {
            addr = stripped.to_string();
        } else if addr.starts_with("https://") {
            return Err("HTTPS not implemented yet");
        }

        // Add port if missing
        if !addr.contains(':') {
            addr.push_str(":80");
        }

        let addr = addr.split("/").next().unwrap();
        // Resolve address
        let addr = if addr.starts_with("localhost") {
            addr.replace("localhost", "127.0.0.1").to_string()
        } else {
            addr.to_string()
        };
        let resolved_addrs: Vec<_> = addr
            .to_socket_addrs()
            .map_err(|_| "Unable to resolve domain: {:?}")?
            .collect();

        let socket_addr = resolved_addrs
            .into_iter()
            .find(|addr| addr.port() != 0 && !addr.ip().is_unspecified())
            .ok_or("No valid address found")?;

        // Connect to server
        let stream = TcpStream::connect_timeout(&socket_addr, Duration::from_secs(5))
            .map_err(|_| "Error connecting to server")?;

        let mut stream = stream;
        let _ = stream.write_all(self.to_string().as_bytes());
        let _ = stream.flush();
        let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));

        let mut raw: Vec<u8> = Vec::new();
        let mut buffer = [0u8; 4096];

        loop {
            match stream.read(&mut buffer) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    raw.extend_from_slice(&buffer[..n]);
                    if n < 4096 {
                        //TODO: write proper response parser
                        break;
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    println!("Read timeout");
                    break;
                }
                Err(e) => return Err("Error reading"),
            }
        }

        Ok(Box::new(HttpResponse::new_raw(raw)))
    }
}

/// Alias to send a request via `request.brew()`.
///
/// Useful for calling as a standalone function.
pub fn brew(direction: &str, request: &mut HttpRequest) -> Result<Box<HttpResponse>, &'static str> {
    request.brew(direction)
}

// pub fn brew_url(url: &str) -> Result<HttpResponse, &'static str> {
//     todo!()
// }

#[cfg(test)]
mod tests {
    use super::super::methods::HttpMethod;
    use super::*;
    #[test]
    fn test_http_request_new() {
        let request = HttpRequest::new(HttpMethod::GET, "/example");
        assert_eq!(request.method, HttpMethod::GET);
        assert_eq!(request.path, "/example");
        assert!(request.args.is_empty());
        assert!(request.headers.is_empty());
        assert_eq!(request.text(), None);
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

    // #[test]
    // fn test_http_request_body() {
    //     let mut request = HttpRequest::new(HttpMethod::POST, "/upload");
    //     request.body("Test body content".to_string());
    //     assert_eq!(request.body, "Test body content");
    // }

    #[test]
    fn test_http_request_to_string() {
        let mut request = HttpRequest::new(HttpMethod::POST, "/resource");
        request.header("Content-Type", "application/json");
        //.body("{\"data\":\"test\"}".to_string());

        let request_string = request.to_string();
        assert!(request_string.contains("POST /resource HTTP/1.1"));
        assert!(request_string.contains("Content-Type: application/json"));
        //assert!(request_string.contains("{\"data\":\"test\"}"));
    }

    // #[test]
    // fn test_http_request_to_string_with_args() {
    //     let mut request = HttpRequest::new(HttpMethod::POST, "/resource");
    //     let _ = request
    //         .header("Content-Type", "application/json")
    //         .arg("key", "value")
    //         .body("{\"data\":\"test\"}".to_string())
    //         .brew("localhost:8080");

    //     let request_string = request.to_string();
    //     assert!(request_string.contains("POST /resource?key=value HTTP/1.1"));
    //     assert!(request_string.contains("Content-Type: application/json"));
    //     assert!(request_string.contains("{\"data\":\"test\"}"));
    // }

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
