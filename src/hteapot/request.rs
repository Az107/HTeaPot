use super::HttpMethod;
use std::collections::HashMap;

#[derive(Clone)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub args: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: String,
}
