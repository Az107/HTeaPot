use super::HttpMethod;
use std::collections::HashMap;

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

pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub args: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: String,
}
