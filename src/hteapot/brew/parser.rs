use super::super::response::BaseResponse;
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

        match self.state {
            State::Body => {
                self.body.append(&mut self.buffer.clone());
                self.buffer.clear();
                if let Some(content_length) = self.response_base.headers.get("content-length") {
                    let content_length = content_length
                        .parse::<usize>()
                        .map_err(|_| "invalid content-length")?;
                    if self.body.len() == content_length {
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
            _ => {}
        }

        while let Some(pos) = self.buffer.windows(2).position(|w| w == b"\r\n") {
            let line = self.buffer.drain(..pos).collect::<Vec<u8>>();
            self.buffer.drain(..2); // remove CRLF
            let line_str = match str::from_utf8(line.as_slice()) {
                Ok(v) => v.to_string(),
                Err(_e) => return Err("No utf-8"),
            };
            match self.state {
                State::Init => {
                    let parts: Vec<&str> = line_str.split(" ").collect();
                    //HTTP/1.1 200 OK
                    if parts.len() < 3 {
                        println!("parts: {:?}", parts);
                        return Err("Invalid response");
                    }
                    let status_str = parts.get(1).ok_or("Invalid status")?;
                    let status = status_str.parse::<u16>().map_err(|_| "Invalid status")?;
                    self.response_base.status =
                        HttpStatus::from_u16(status).map_err(|_| "Invalid status")?;
                    self.state = State::Headers;
                }
                State::Headers => {
                    if line.is_empty() {
                        self.state = State::Body;
                        continue;
                    }
                    let (k, v) = line_str.split_once(":").ok_or("Invalid header")?;
                    let k = k.trim();
                    let v = v.trim();
                    self.response_base.headers.insert(k, v);
                }
                State::Body => {
                    self.body.append(&mut line.clone());
                    if let Some(content_length) = self.response_base.headers.get("content-length") {
                        let content_length = content_length
                            .parse::<usize>()
                            .map_err(|_| "invalid content-length")?;
                        if self.body.len() == content_length {
                            self.state = State::Finish;
                        }
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
