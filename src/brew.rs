// Written by Alberto Ruiz 2024-04-08
// This is the HTTP client module, it will handle the requests and responses

use std::{
    io::{Read, Write},
    net::TcpStream,
};

struct Url {
    scheme: String,
    domain: String,
    path: String,
    port: String,
}

fn parse_url(url: &str) -> Result<Url, &str> {
    let url_parts = url.split(":").collect::<Vec<&str>>();
    let prefix = url_parts[0];
    let domain_path = url_parts[1].trim_start_matches("//");
    let port = if url_parts.len() == 3 {
        url_parts[2]
    } else {
        match prefix {
            "tea" => "1234",
            "https" => "443",
            "http" => "80",
            _ => "80",
        }
    };
    let (domain, path) = domain_path.split_once('/').unwrap();
    Ok(Url {
        scheme: prefix.to_string(),
        domain: domain.to_string(),
        path: path.to_string(),
        port: port.to_string(),
    })
}

pub fn fetch(url: &str) -> Result<Vec<u8>, &str> {
    let url = parse_url(url);
    if url.is_err() {
        return Err("Error parsing url");
    }
    let url = url.unwrap();
    if url.scheme == "https" {
        return Err("not supported yet");
    }

    let client = TcpStream::connect(format!("{}:{}", url.domain, url.port));
    if client.is_err() {
        return Err("Error fetching");
    }
    let mut client = client.unwrap();
    let http_request = format!("GET /{} HTTP/1.1\r\nHost: {}\r\n\r\n", url.path, url.domain);
    client.write(http_request.as_bytes()).unwrap();
    let mut full_buffer: Vec<u8> = Vec::new();
    let mut buffer = [0; 1024];
    loop {
        match client.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                if n == 0 {
                    break;
                }
                full_buffer.extend(buffer.iter().cloned());
                if buffer.last().unwrap() == &0 {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    Ok(full_buffer)
}
