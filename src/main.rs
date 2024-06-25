pub mod hteapot;
mod config;
mod brew;


use std::fs;
use std::io;

use hteapot::Hteapot;
use hteapot::HttpStatus;
use brew::fetch;





fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let config = if args.len() > 1 {
        config::Config::load_config(&args[1])
    } else {
        config::Config::new_default()
    };
    let mut server = Hteapot::new(config.host.as_str(), config.port);
    println!("Server started at http://{}:{}", config.host, config.port);
    server.listen( move |req| {
        //println!("Request: {:?}", req.path);
        let path = if req.path.ends_with("/") {
            let mut path = req.path.clone();
            path.push_str(&config.index);
            path
        } else {
            req.path.clone()
        };
        if config.proxy_rules.contains_key(&req.path) {
            println!("Proxying to: {}", config.proxy_rules.get(&req.path).unwrap());
            let url = config.proxy_rules.get(&req.path).unwrap();
            return match fetch(url) {
                Ok(response) => {
                    response.into()
                },
                Err(err) => {
                    Hteapot::response_maker(HttpStatus::InternalServerError, err.as_bytes(), None)
                }
            }
        }
        let path = format!("./{}/{}",config.root, path);
        let content = fs::read(path);
        match content {
            Ok(content) => {
                return Hteapot::response_maker(HttpStatus::OK,&content, headers!("Connection" => "close"));
            },
            Err(e) => {
                match e.kind() {
                    io::ErrorKind::NotFound => {
                        return Hteapot::response_maker(HttpStatus::NotFound, "<h1> 404 Not Found </h1>", headers!("Content-Type" => "text/html", "Server" => "HteaPot"));
                    },
                    _ => {
                        return Hteapot::response_maker(HttpStatus::InternalServerError, "<h1> 500 Internal Server Error </h1>", headers!("Content-Type" => "text/html", "Server" => "HteaPot"));
                    }
                }
            }
        }
    });
}