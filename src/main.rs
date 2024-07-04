mod logger;
pub mod hteapot;
mod config;
mod brew;


use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::BufWriter;
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::Mutex;
use std::time;
use std::time::SystemTime;

use hteapot::Hteapot;
use hteapot::HttpStatus;
use brew::fetch;
use logger::Logger;


fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let config = if args.len() > 1 {
        config::Config::load_config(&args[1])
    } else {
        config::Config::new_default()
    };
    let logger = Mutex::new(Logger::new(io::stdout()));
    let cache: Mutex<HashMap<String, (Vec<u8>, u64)>> = Mutex::new(HashMap::new());
    let server = Hteapot::new_threaded(config.host.as_str(), config.port,config.threads);
    logger.lock().expect("this doesnt work :C").msg(format!("Server started at http://{}:{}", config.host, config.port));
    if config.cache {
        logger.lock().expect("this doesnt work :C").msg("Cache Enabled".to_string());
    }

    server.listen( move |req| {
        //let mut logger = Logger::new(io::stdout());
        logger.lock().expect("this doesnt work :C").msg(format!("Request {} {}",req.method.to_str(), req.path));
        let path = if req.path.ends_with("/") {
            let mut path = req.path.clone();
            path.push_str(&config.index);
            path
        } else {
            req.path.clone()
        };
        if config.proxy_rules.contains_key(&req.path) {
            logger.lock().expect("this doesnt work :C").msg(format!("Proxying to: {}", config.proxy_rules.get(&req.path).unwrap()));
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
        let cache_result = 
        {
            if config.cache {
                let cache = cache.lock();
                if cache.is_err() {
                    None
                }else {
                    let cache = cache.unwrap();
                    let r = cache.get(&path);
                    match r {
                        Some(r) => Some(r.clone()),
                        None => None
                    }
                }
            } else {
                None
            }
        };
        let mut is_cache = false;
        let content: Result<Vec<u8>, _> = if cache_result.is_some() {
            let (content,ttl) = cache_result.unwrap();
            let now = SystemTime::now();
            let since_epoch = now.duration_since(time::UNIX_EPOCH).expect("Time went backwards");
            let secs = since_epoch.as_secs();
            if secs > ttl {
                fs::read(&path)
            } else {
                is_cache = true;
                Ok(content)
            }
        } else {
            fs::read(&path)
        };
        match content {
            Ok(content) => {

                if config.cache {
                    let cache = cache.lock();
                    if cache.is_ok() && is_cache {
                        let mut cache = cache.unwrap();
                        let now = SystemTime::now();
                        let since_epoch = now.duration_since(time::UNIX_EPOCH).expect("Time went backwards");
                        let secs = since_epoch.as_secs() + config.cache_ttl;
                        cache.insert(path,(content.clone(),secs));
                    }
                    
                }
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