// Written by Alberto Ruiz 2025-01-01
//
// This module defines the HTTP request structure and a streaming builder for parsing raw input.
// While the core functionality is usable, there are known limitations:
// - No support for chunked transfer encoding
// - Partial header validation
// - No URI normalization or encoding
//

use super::HttpHeaders;
use super::HttpMethod;
use std::hash::Hash;
use std::{cmp::min, collections::HashMap, net::TcpStream, str};

const MAX_HEADER_SIZE: usize = 1024 * 16;
const MAX_HEADER_COUNT: usize = 100;

/// Represents a parsed HTTP request.
///
/// Contains method, path, optional query arguments, headers, body, and a stream (for low-level access).
#[derive(Debug)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub args: HashMap<String, String>,
    pub headers: HttpHeaders,
    pub body: Vec<u8>,
    stream: Option<TcpStream>,
}

impl Hash for HttpRequest {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.method.hash(state);
        self.path.hash(state);
        // self.args.hash(state);
        // self.headers.hash(state);
        self.body.hash(state);
    }
}

impl PartialEq for HttpRequest {
    fn eq(&self, other: &Self) -> bool {
        let same_method = self.method == other.method;
        let same_path = self.path == other.path;
        let same_body = self.body == other.body;
        let same_args = other.args == self.args;
        let same_headers = self.headers == other.headers;
        return same_method && same_path && same_body && same_args && same_headers;
    }
}

impl Eq for HttpRequest {}

impl HttpRequest {
    /// Creates a new HTTP request with the given method and path.
    pub fn new(method: HttpMethod, path: &str) -> Self {
        return HttpRequest {
            method,
            path: path.to_string(),
            args: HashMap::new(),
            headers: HttpHeaders::new(),
            body: Vec::new(),
            stream: None,
        };
    }

    /// Returns a blank default request (empty method/path/headers).
    pub fn default() -> Self {
        HttpRequest {
            method: HttpMethod::Other(String::new()),
            path: String::new(),
            args: HashMap::new(),
            headers: HttpHeaders::new(),
            body: Vec::new(),
            stream: None,
        }
    }

    /// Returns a blank default request (empty method/path/headers).
    pub fn clone(&self) -> Self {
        return HttpRequest {
            method: self.method.clone(),
            path: self.path.clone(),
            args: self.args.clone(),
            headers: self.headers.clone(),
            body: self.body.clone(),
            stream: None,
        };
    }

    /// Attaches a raw TCP stream to this request.
    pub fn set_stream(&mut self, stream: TcpStream) {
        self.stream = Some(stream);
    }

    /// Attempts to decode the body as UTF-8 and return it as text.
    pub fn text(&self) -> Option<String> {
        if self.body.len() == 0 {
            return None;
        }
        let body = match str::from_utf8(self.body.as_slice()) {
            Ok(v) => Some(v.to_string()),
            Err(_e) => None,
        };
        return body;
    }
}

/// Builder for incrementally parsing a raw HTTP request.
///
/// This is useful when reading from a stream (e.g., TCP) in chunks.
pub struct HttpRequestBuilder {
    request: HttpRequest,
    buffer: Vec<u8>,
    state: State,
    body_size: usize,
    chunked: bool,
}

#[derive(PartialEq)]
enum State {
    Init,
    Headers,
    Body,
    Finish,
}

impl HttpRequestBuilder {
    /// Creates a new builder in the initial state.
    pub fn new() -> Self {
        return HttpRequestBuilder {
            request: HttpRequest {
                method: HttpMethod::GET,
                path: String::new(),
                args: HashMap::new(),
                headers: HttpHeaders::new(),
                body: Vec::new(),
                stream: None,
            },
            chunked: false,
            state: State::Init,
            body_size: 0,
            buffer: Vec::new(),
        };
    }

    /// Returns the built request if parsing is complete.
    pub fn get(&self) -> Option<HttpRequest> {
        match self.state {
            State::Finish => Some(self.request.clone()),
            _ => None,
        }
    }

    /// Reads bytes into the request body based on `Content-Length`.
    fn read_body_len(&mut self) -> Option<()> {
        let body_left = self.body_size.saturating_sub(self.request.body.len());

        let body_left = self.body_size.saturating_sub(self.request.body.len());

        if body_left > 0 {
            return None;
        } else {
            return Some(());
        }
    }

    /// Placeholder for future support of chunked body parsing.
    fn _read_body_chunk(&mut self) -> Option<()> {
        //TODO: this will support chunked body in the future
        todo!()
    }

    /// Main entry point for reading the request body.
    fn read_body(&mut self) -> Option<()> {
        return self.read_body_len();
    }

    pub fn done(&self) -> bool {
        self.state == State::Finish
    }

    /// Feeds a chunk of bytes into the builder.
    ///
    /// This function may return an error if the header is too large or malformed.
    pub fn append(&mut self, chunk: Vec<u8>) -> Result<(), &'static str> {
        self.buffer.extend(chunk);

        while !self.buffer.is_empty() {
            match self.state {
                State::Init => {
                    let line = get_line(&mut self.buffer);
                    if line.is_none() {
                        if self.buffer.len() >= MAX_HEADER_SIZE {
                            return Err("Entity Too Large");
                        }
                        return Ok(());
                    }
                    let line = line.unwrap();
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() != 3 {
                        return Err("Invalid method + path + version request");
                    }
                    self.request.method = HttpMethod::from_str(parts[0]);
                    let path_parts: Vec<&str> = parts[1].split('?').collect();
                    self.request.path = path_parts[0].to_string();
                    if path_parts.len() > 1 {
                        self.request.args = path_parts[1]
                            .split('&')
                            .filter_map(|pair| {
                                let kv: Vec<&str> = pair.split('=').collect();
                                if kv.len() == 2 {
                                    Some((kv[0].to_string(), kv[1].to_string()))
                                } else {
                                    None
                                }
                            })
                            .collect();
                    }
                    self.state = State::Headers;
                }
                State::Headers => {
                    let line = get_line(&mut self.buffer);
                    if line.is_none() {
                        return Ok(());
                    }
                    let line = line.unwrap();
                    if line.is_empty() {
                        self.state = if self.body_size == 0 && !self.chunked {
                            State::Finish
                        } else {
                            State::Body
                        };
                        continue;
                    }
                    if self.request.headers.len() > MAX_HEADER_COUNT || line.len() > MAX_HEADER_SIZE
                    {
                        return Err("Header number exceed allowed");
                    }

                    let (key, value) = line.split_once(':').ok_or("Invalid Header")?;
                    let key = key.trim();
                    let value = value.trim();
                    if key.to_lowercase() == "content-length" {
                        if self.request.headers.get("content-length").is_some()
                            || self.request.headers.get("Transfer-Encoding")
                                == Some(&"chunked".to_string())
                        {
                            continue;
                        }
                        self.body_size = value
                            .parse::<usize>()
                            .map_err(|_| "invalid content-length")?;
                    }
                    if key.to_lowercase() == "transfer-encoding"
                        && value.to_lowercase() == "chunked"
                    {
                        if self.request.headers.get("content-length").is_some() {
                            continue;
                        }
                        self.chunked = true;
                    }
                    self.request.headers.insert(key, value);
                }
                State::Body => {
                    let body_left = self.body_size - self.request.body.len();
                    if body_left > 0 {
                        let to_take = min(body_left, self.buffer.len());
                        let to_append = self.buffer.drain(..to_take);
                        let to_append = to_append.as_slice();
                        self.request.body.extend_from_slice(to_append);
                    }
                    if self.chunked {
                        if self.body_size != 0 {
                            let empty = get_line(&mut self.buffer);
                            if empty.is_none() {
                                return Ok(());
                            }
                        }
                        let size = get_line(&mut self.buffer);
                        if size.is_none() {
                            return Ok(());
                        }
                        let size = size.unwrap();
                        let size = size.strip_prefix("0x").unwrap_or(&size);
                        let size =
                            i64::from_str_radix(size, 16).map_err(|_| "Invalud chunk size")?;
                        if size == 0 {
                            self.state = State::Finish;
                            return Ok(());
                        }
                        self.body_size += size as usize;
                    } else {
                        self.state = State::Finish;
                        return Ok(());
                    }
                }
                State::Finish => return Ok(()),
            }
        }

        Ok(())
    }
}

fn get_line(buffer: &mut Vec<u8>) -> Option<String> {
    if let Some(pos) = buffer.windows(2).position(|w| w == b"\r\n") {
        let line = buffer.drain(..pos).collect::<Vec<u8>>();
        buffer.drain(..2); // remove CRLF
        return match str::from_utf8(line.as_slice()) {
            Ok(v) => Some(v.to_string()),
            Err(_e) => None,
        };
    }
    None
}

#[cfg(test)]
#[test]
fn basic_request() {
    // Placeholder test — add real body/header parsing test here.
    let buffer = "GET / HTTP/1.1\r\n\r\n".as_bytes().to_vec();
    let mut request_builder = HttpRequestBuilder::new();
    let done = request_builder.append(buffer);
    assert!(done.is_ok());
    let request = request_builder.get();
    assert!(request.is_some());
    let request = request.unwrap();
    assert!(request.path == "/");
    assert!(request.method == HttpMethod::GET);
    assert!(request.headers.len() == 0);
}

#[cfg(test)]
#[test]
fn basic_request_headers() {
    // Placeholder test — add real body/header parsing test here.
    let buffer = "GET / HTTP/1.1\r\nHost: test\r\n\r\n".as_bytes().to_vec();
    let mut request_builder = HttpRequestBuilder::new();
    let done = request_builder.append(buffer);
    assert!(done.is_ok());
    let request = request_builder.get();
    assert!(request.is_some());
    let request = request.unwrap();
    assert!(request.path == "/");
    assert!(request.method == HttpMethod::GET);
    assert!(request.headers.len() == 1);
}

#[cfg(test)]
#[test]
fn post_request() {
    // Placeholder test — add real body/header parsing test here.
    let buffer = "POST / HTTP/1.1\r\ncontent-length: 4\r\n\r\nhello\r\n"
        .as_bytes()
        .to_vec();
    let mut request_builder = HttpRequestBuilder::new();
    let done = request_builder.append(buffer);
    assert!(done.is_ok());
    let request = request_builder.get();
    assert!(request.is_some());
    let request = request.unwrap();
    assert!(request.path == "/");
    assert!(request.method == HttpMethod::POST);
    assert!(request.headers.len() == 1);
}
