use super::HttpMethod;
use std::{collections::HashMap, net::TcpStream, str};

const MAX_HEADER_SIZE: usize = 1024 * 16;

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
            method: HttpMethod::Other(String::new()),
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
    header_done: bool,
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
            header_done: false,
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

    fn read_body(&mut self) -> Option<()> {
        self.request.body.append(&mut self.buffer.clone());
        if self.request.body.len() == self.body_size {
            self.done = true;
            return Some(());
        } else {
            return None;
        }
    }

    pub fn append(&mut self, buffer: Vec<u8>) -> Result<Option<HttpRequest>, &'static str> {
        if !self.header_done && self.buffer.len() > MAX_HEADER_SIZE {
            return Err("Entity Too large");
        }
        self.buffer.extend(buffer);
        self.buffer.retain(|&b| b != 0);
        if self.header_done {
            match self.read_body() {
                Some(_) => return Ok(Some(self.request.clone())),
                None => return Ok(None),
            }
        }
        while let Some(pos) = self.buffer.windows(2).position(|w| w == b"\r\n") {
            let line = self.buffer.drain(..pos).collect::<Vec<u8>>();
            self.buffer.drain(..2);

            let line_str = match str::from_utf8(line.as_slice()) {
                Ok(v) => v.to_string(),
                Err(_e) => return Err("No utf-8"),
            };

            if self.request.path.is_empty() {
                let parts: Vec<&str> = line_str.split_whitespace().collect();
                if parts.len() < 2 {
                    return Ok(None);
                }

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
            } else if !line_str.is_empty() {
                if let Some((key, value)) = line_str.split_once(":") {
                    let key = key.trim().to_lowercase();
                    let value = value.trim();
                    if key == "content-length" {
                        if self.request.headers.get("content-length").is_some()
                            || self
                                .request
                                .headers
                                .get("transfer-encoding")
                                .map(|te| te == "chunked")
                                .unwrap_or(false)
                        {
                            return Err("Duplicated content-length");
                        }
                        self.body_size = value.parse().unwrap_or(0);
                    }
                    self.request
                        .headers
                        .insert(key.to_string(), value.to_string());
                }
            } else {
                self.header_done = true;
                match self.read_body() {
                    Some(_) => return Ok(Some(self.request.clone())),
                    None => return Ok(None),
                }
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
#[test]
fn basic_request() {}
