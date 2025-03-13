use super::HttpMethod;
use std::{collections::HashMap, net::TcpStream, str};

pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub args: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    stream: Option<TcpStream>,
}

impl HttpRequest {
    pub fn new(method: HttpMethod, path: &str) -> Self {
        return HttpRequest {
            method,
            path: path.to_string(),
            args: HashMap::new(),
            headers: HashMap::new(),
            body: Vec::new(),
            stream: None,
        };
    }

    pub fn default() -> Self {
        HttpRequest {
            method: HttpMethod::GET,
            path: String::new(),
            args: HashMap::new(),
            headers: HashMap::new(),
            body: Vec::new(),
            stream: None,
        }
    }

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

    // pub fn body(&mut self) -> Option<Vec<u8>> {
    //     if self.has_body() {
    //         let mut stream = self.stream.as_ref().unwrap();
    //         let content_length = self.headers.get("Content-Length")?;
    //         let content_length: usize = content_length.parse().unwrap();
    //         if content_length > self.body.len() {
    //             let _ = stream.flush();
    //             let mut total_read = 0;
    //             self.body.resize(content_length, 0);
    //             while total_read < content_length {
    //                 match stream.read(&mut self.body[total_read..]) {
    //                     Ok(0) => {
    //                         break;
    //                     }
    //                     Ok(n) => {
    //                         total_read += n;
    //                     }
    //                     Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
    //                         continue;
    //                     }
    //                     Err(_e) => {
    //                         break;
    //                     }
    //                 }
    //             }
    //         }

    //         Some(self.body.clone())
    //     } else {
    //         None
    //     }
    // }

    pub fn set_stream(&mut self, stream: TcpStream) {
        self.stream = Some(stream);
    }

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

pub struct HttpRequestBuilder {
    request: HttpRequest,
    buffer: Vec<u8>,
    body_size: usize,
    pub done: bool,
}

impl HttpRequestBuilder {
    pub fn new() -> Self {
        return HttpRequestBuilder {
            request: HttpRequest {
                method: HttpMethod::GET,
                path: String::new(),
                args: HashMap::new(),
                headers: HashMap::new(),
                body: Vec::new(),
                stream: None,
            },
            body_size: 0,
            buffer: Vec::new(),
            done: false,
        };
    }

    pub fn get(&self) -> Option<HttpRequest> {
        if self.done {
            return Some(self.request.clone());
        } else {
            None
        }
    }

    pub fn append(&mut self, buffer: Vec<u8>) -> Option<HttpRequest> {
        self.buffer.extend(buffer);
        self.buffer.retain(|&b| b != 0);
        while let Some(pos) = self.buffer.windows(2).position(|w| w == b"\r\n") {
            let line = self.buffer.drain(..pos).collect::<Vec<u8>>();
            self.buffer.drain(..2);

            let line_str = String::from_utf8_lossy(&line);

            if self.request.path.is_empty() {
                let parts: Vec<&str> = line_str.split_whitespace().collect();
                if parts.len() < 2 {
                    return None;
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
            } else if !line_str.is_empty() {
                if let Some((key, value)) = line_str.split_once(": ") {
                    if key.to_lowercase() == "content-length" {
                        self.body_size = value.parse().unwrap_or(0);
                    }
                    self.request
                        .headers
                        .insert(key.to_string(), value.to_string());
                }
            }
        }
        self.request.body.append(&mut self.buffer.clone());
        if self.request.body.len() == self.body_size {
            self.done = true;
            return Some(self.request.clone());
        }
        None
    }
}
