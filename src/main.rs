mod hteapot;
mod config;
use std::fs;

use hteapot::HteaPot;

use crate::hteapot::HttpStatus;


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