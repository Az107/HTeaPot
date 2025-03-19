use super::HttpStatus;
use super::VERSION;
use std::collections::HashMap;

const BUFFER_SIZE: usize = 1024;

pub struct HttpResponse {
    pub status: HttpStatus,
    pub headers: HashMap<String, String>,
    pub content: Vec<u8>,
    raw: Option<Vec<u8>>,
    is_raw: bool,
    index: usize,
}

#[derive(Debug)]
pub enum IterError {
    WouldBlock,
    Finished,
}

pub trait HttpResponseConsumer {
    fn next(&mut self) -> Result<Vec<u8>, IterError>;
    fn peek(&mut self) -> Result<Vec<u8>, IterError>; //TODO: come up with better solution
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
            index: 0,
        }
    }

    pub fn new_raw(raw: Vec<u8>) -> Self {
        HttpResponse {
            status: HttpStatus::IAmATeapot,
            headers: HashMap::new(),
            content: vec![],
            raw: Some(raw),
            is_raw: true,
            index: 0,
        }
    }

    pub fn is_raw(&self) -> bool {
        self.is_raw
    }

    pub fn to_bytes(&mut self) -> Vec<u8> {
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
        response.append(&mut self.content);
        response.push(0x0D); // Carriage Return
        response.push(0x0A); // Line Feed
        response
    }
}

impl HttpResponseConsumer for HttpResponse {
    fn next(&mut self) -> Result<Vec<u8>, IterError> {
        let byte_chunk = self.peek()?;
        self.index += 1;
        return Ok(byte_chunk);
    }

    fn peek(&mut self) -> Result<Vec<u8>, IterError> {
        if self.raw.is_none() {
            self.raw = Some(self.to_bytes());
        }
        let raw = self.raw.as_ref().unwrap();
        let mut raw = raw.chunks(BUFFER_SIZE).skip(self.index);
        // println!("{}/{}",self.)
        let byte_chunk = raw.next().ok_or(IterError::Finished)?.to_vec();
        return Ok(byte_chunk);
    }
}

pub struct EmptyHttpResponse {}

impl EmptyHttpResponse {}
impl HttpResponseConsumer for EmptyHttpResponse {
    fn next(&mut self) -> Result<Vec<u8>, IterError> {
        Err(IterError::Finished)
    }

    fn peek(&mut self) -> Result<Vec<u8>, IterError> {
        Err(IterError::Finished)
    }
}

pub struct StreamedResponse {}

impl StreamedResponse {
    pub fn new() -> Result<Self, &'static str> {
        Err("Request does not have a stream")
    }
}
