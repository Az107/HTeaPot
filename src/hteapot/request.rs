use super::HttpMethod;
use std::collections::HashMap;

// WIP 🚧
struct Body {
    data: Vec<u8>,
}

impl Body {
    pub fn text() {}
}

pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub args: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: String,
}

// Parse the request
pub fn request_parser(request: String) -> Result<HttpRequest, String> {
    let mut lines = request.lines();
    let first_line = lines.next();
    if first_line.is_none() {
        println!("{}", request);
        return Err("Invalid request".to_string());
    }
    let first_line = first_line.unwrap();
    let mut words = first_line.split_whitespace();
    let method = words.next();
    if method.is_none() {
        return Err("Invalid method".to_string());
    }
    let method = method.unwrap();
    let path = words.next();
    if path.is_none() {
        return Err("Invalid path".to_string());
    }
    let mut path = path.unwrap().to_string();
    let mut headers: HashMap<String, String> = HashMap::new();
    loop {
        let line = lines.next();
        if line.is_none() {
            break;
        }
        let line = line.unwrap();
        if line.is_empty() {
            break;
        }
        let mut parts = line.split(": ");
        let key = parts.next().unwrap().to_string();
        let value = parts.next().unwrap();
        headers.insert(key, value.to_string());
    }
    let body = lines
        .collect::<Vec<&str>>()
        .join("")
        .trim()
        .trim_end_matches(char::from(0))
        .to_string();
    let mut args: HashMap<String, String> = HashMap::new();
    //remove http or https from the path
    if path.starts_with("http://") {
        path = path.trim_start_matches("http://").to_string();
    } else if path.starts_with("https://") {
        path = path.trim_start_matches("https://").to_string();
    }
    //remove the host name if present
    if !path.starts_with("/") {
        //remove all the characters until the first /
        let mut parts = path.split("/");
        parts.next();
        path = parts.collect::<Vec<&str>>().join("/");
        //add / to beggining
        path = format!("/{}", path);
    }

    if path.contains('?') {
        let _path = path.clone();
        let mut parts = _path.split('?');
        path = parts.next().unwrap().to_string();
        let query = parts.next().unwrap();
        let query_parts: Vec<&str> = query.split('&').collect();
        for part in query_parts {
            let mut parts = part.split('=');
            let key = parts.next().unwrap().to_string();
            let value = parts.next().unwrap_or("").to_string().replace("%22", "\"");
            args.insert(key, value);
        }
    }

    Ok(HttpRequest {
        method: HttpMethod::from_str(method),
        path: path.to_string(),
        args,
        headers,
        body: body.trim_end().to_string(),
    })
}
