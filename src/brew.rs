// Written by Alberto Ruiz 2024-04-08
// This is the HTTP client module, it will handle the requests and responses

use std::{io::{Read, Write}, net::TcpStream};


struct Url {
  scheme: String,
  domain: String,
  port: String
}


fn parse_url(url: &str) -> Result<Url,&str> {
  let url_parts = url.split(":").collect::<Vec<&str>>();
  let prefix = url_parts[0];
  let domain = url_parts[1].trim_start_matches("//");
  let port = if url_parts.len() == 3 {
    url_parts[2]
  } else {
    match prefix {
      "https" => "443",
      "http" => "80",
      _ => "80"
    }
  };
  Ok(Url {
    scheme: prefix.to_string(),
    domain: domain.to_string(),
    port: port.to_string()
  })
}

pub fn fetch(url: &str) -> Result<String,&str> {
  let url = parse_url(url);
  if url.is_err() { return Err("Error parsing url")}
  let url = url.unwrap();
  if url.scheme == "https" {
    return Err("not supported yet");
  }
  let client = TcpStream::connect(format!("{}:{}",url.domain,url.port));
  if client.is_err() {
    return Err("Error fetching");
  }
  let mut client = client.unwrap();
  let http_request = format!("GET / HTTP/1.1\r\nHost: {}\r\n\r\n", url.domain);
  client.write(http_request.as_bytes()).unwrap();
  let mut response = String::new();
  let mut buffer = [0; 1024];
  loop {
      match client.read(&mut buffer) {
          Ok(0) => break,
          Ok(n) => {
              response.push_str(std::str::from_utf8(&buffer[..n]).unwrap());
              if response.ends_with("\n") {break} //TODO: break when size == header
              
          },
          Err(_) => break
      }
  }
  Ok(response)
}


