mod hteapot;
mod config;
use std::{fs, io::{Read, Write}, net::{TcpListener, TcpStream}};

use hteapot::HteaPot;

use crate::hteapot::HttpStatus;


fn fetch(url: &str) -> String {
    let url = url.trim_start_matches("http://");
    let mut client = TcpStream::connect(url).unwrap();
    let http_request = format!("GET / HTTP/1.1\r\nHost: {}\r\n\r\n", url);
    client.write(http_request.as_bytes()).unwrap();
    let mut response = String::new();
    let mut buffer = [0; 1024];
    loop {
        match client.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                response.push_str(std::str::from_utf8(&buffer[..n]).unwrap());
                println!("{}: {}", n,response);
                if response.ends_with("\n") {break} //TODO: break when size == header
                
            },
            Err(_) => break
        }
    }
    response
}


fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let config = if args.len() > 1 {
        config::config::load_config(&args[1])
    } else {
        config::config::new_default()
    };
    let server = HteaPot::new(config.host.as_str(), config.port);
    println!("Server started at http://{}:{}", config.host, config.port);
    server.listen(|req| {
        println!("Request: {:?}", req.path);
        let path = if req.path.ends_with("/") {
            let mut path = req.path.clone();
            path.push_str(&config.index);
            path
        } else {
            req.path.clone()
        };
        if config.proxyRules.contains_key(&req.path) {
            println!("Proxying to: {}", config.proxyRules.get(&req.path).unwrap());
            let url = config.proxyRules.get(&req.path).unwrap();
            return fetch(url);
            //return HteaPot::response_maker(HttpStatus::OK, "", Some(url.to_string()));
        }
        let path = format!("./{}/{}",config.root, path);
        let content = fs::read_to_string(path);
        match content {
            Ok(content) => {
                return HteaPot::response_maker(HttpStatus::OK, &content, None);
            },
            Err(_) => {
                return HteaPot::response_maker(HttpStatus::NotFound, "<h1> 404 Not Found </h1>", headers!("Content-Type" => "text/html", "Server" => "HteaPot"));
            }
        }
    });
}