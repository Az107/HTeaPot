use super::super::response::BaseResponse;
use super::super::response::HttpResponseCommon;
use super::super::{HttpHeaders, HttpStatus};
use crate::HttpResponse;

use std::usize;

enum State {
    Init,
    Headers,
    Body,
    Finish,
}

pub struct HttpResponseBuilder {
    body: Vec<u8>,
    body_size: usize,
    response_base: BaseResponse,
    buffer: Vec<u8>,
    state: State,
}

impl HttpResponseBuilder {
    pub fn new() -> HttpResponseBuilder {
        HttpResponseBuilder {
            response_base: BaseResponse {
                status: HttpStatus::IAmATeapot,
                headers: HttpHeaders::new(),
            },
            body_size: 0,
            body: Vec::new(),
            buffer: Vec::new(),
            state: State::Init,
        }
    }

    pub fn get(&self) -> Option<HttpResponse> {
        match self.state {
            State::Finish => {
                let response =
                    HttpResponse::new_with_base(self.response_base.clone(), self.body.clone());
                Some(response)
            }
            _ => None,
        }
    }

    pub fn append(&mut self, chunk: &[u8]) -> Result<bool, &'static str> {
        self.buffer.extend_from_slice(chunk);

        while !self.buffer.is_empty() {
            match self.state {
                State::Init => {
                    if let Some(line) = get_line(&mut self.buffer) {
                        let parts: Vec<&str> = line.split(" ").collect();
                        if parts.len() < 3 {
                            return Err("Invalid response");
                        }
                        let status_str = parts.get(1).ok_or("Invalid status")?;
                        let status = status_str.parse::<u16>().map_err(|_| "Invalid status")?;
                        self.response_base.status =
                            HttpStatus::from_u16(status).map_err(|_| "Invalid status")?;
                        self.state = State::Headers;
                    } else {
                        return Ok(false);
                    }
                }
                State::Headers => {
                    if let Some(line) = get_line(&mut self.buffer) {
                        if line.is_empty() {
                            self.state = if self.body_size == 0 {
                                State::Finish
                            } else {
                                State::Body
                            };
                            continue;
                        }
                        let (key, value) = line.split_once(":").ok_or("Invalid header")?;
                        let key = key.trim();
                        let value = value.trim();
                        if key.to_lowercase() == "content-length" {
                            self.body_size = value
                                .parse::<usize>()
                                .map_err(|_| "invalid content-length")?;
                        }
                        self.response_base.headers.insert(key, value);
                    } else {
                        return Ok(false);
                    }
                }
                State::Body => {
                    self.body.extend_from_slice(&mut self.buffer.as_slice());
                    self.buffer.clear();
                    if let Some(content_length) = self.response_base.headers.get("content-length") {
                        let content_length = content_length
                            .parse::<usize>()
                            .map_err(|_| "invalid content-length")?;
                        if self.body.len() >= content_length {
                            self.state = State::Finish;
                            return Ok(true);
                        } else {
                            return Ok(false);
                        }
                    } else {
                        //TODO: handle chunked
                        self.state = State::Finish;
                        return Ok(true);
                    }
                }
                State::Finish => {
                    return Ok(true);
                }
            }
        }

        Ok(false)
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
fn basic_response() {
    // Placeholder test — add real body/header parsing test here.

    let buffer = "HTTP/1.1 204 No Content\r\n\r\n".as_bytes().to_vec();
    let mut response_builder = HttpResponseBuilder::new();
    let done = response_builder.append(buffer.as_slice());
    assert!(done.is_ok());
    let response = response_builder.get();
    assert!(response.is_some());
    let mut response = response.unwrap();
    let response_base = response.base();
    assert!(response_base.status == HttpStatus::NoContent);
    assert!(response_base.headers.len() == 0);
    assert!(response.content.len() == 0);
}
