mod brew;
mod cache;
mod config;
pub mod hteapot;
mod logger;

use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;

use brew::fetch;
use cache::Cache;
use config::Config;
use hteapot::{Hteapot, HttpResponse, HttpStatus};

use logger::Logger;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn is_proxy(config: &Config, path: String) -> Option<String> {
    for proxy_path in config.proxy_rules.keys() {
        let mut proxy_path_f = proxy_path.clone();
        if !proxy_path.ends_with("/") {
            proxy_path_f = format!("{}/", proxy_path);
        }
        println!("{} -- {}", proxy_path_f, path);
        let path_proxy = path.strip_prefix(&proxy_path_f);
        if path_proxy.is_some() {
            let path_proxy = path_proxy.unwrap();
            let url = config.proxy_rules.get(proxy_path).unwrap();
            let separator = if path_proxy.starts_with('/') || url.ends_with('/') {
                ""
            } else {
                "/"
            };
            let url = format!("{}{}{}", url, separator, path_proxy);
            return Some(url);
        }
    }
    None
}

fn serve_proxy(proxy_url: String) -> HttpResponse {
    let raw_response = fetch(&proxy_url);
    match raw_response {
        Ok(raw) => HttpResponse::new_raw(raw),
        Err(_) => HttpResponse::new(HttpStatus::NotFound, "not found", None),
    }
}

fn get_mime_tipe(path: &String) -> String {
    let extension = Path::new(path.as_str())
        .extension()
        .unwrap()
        .to_str()
        .unwrap();
    let mimetipe = match extension {
        "js" => "text/javascript",
        "json" => "application/json",
        "css" => "text/css",
        "html" => "text/html",
        "ico" => "image/x-icon",
        _ => "text/plain",
    };

    mimetipe.to_string()
}

fn serve_file(path: &String) -> Option<Vec<u8>> {
    let r = fs::read(path);
    if r.is_ok() {
        Some(r.unwrap())
    } else {
        None
    }
}

#[cfg(feature = "cgi")]
fn serve_cgi(program: &String, path: &String) -> Result<Vec<u8>, &'static str> {
    let output = Command::new(program).arg(&path).output();
    match output {
        Ok(output) => Ok(output.stdout),
        Err(_) => Err("Error runing command"),
    }
}

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let mut serving_path = None;
    if args.len() >= 2 {
        match args[1].as_str() {
            "--help" | "-h" => {
                println!("Hteapot {}", VERSION);
                println!("usage: {} <config file>", args[0]);
                return;
            }
            "--version" | "-v" => {
                println!("Hteapot {}", VERSION);
                return;
            }
            "--serve" | "-s" => {
                serving_path = Some(args.get(2).unwrap().clone());
            }
            _ => (),
        };
    }

    let config = if args.len() == 2 {
        config::Config::load_config(&args[1])
    } else if serving_path.is_some() {
        let serving_path_str = serving_path.unwrap();
        let serving_path_str = serving_path_str.as_str();
        let serving_path = Path::new(serving_path_str);
        let mut c = config::Config::new_default();
        c.host = "0.0.0.0".to_string();
        if serving_path.is_dir() {
            c.root = serving_path.to_str().unwrap_or_default().to_string();
        } else {
            c.index = serving_path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap_or_default()
                .to_string();
            c.root = serving_path
                .parent()
                .unwrap_or(Path::new("./"))
                .to_str()
                .unwrap_or_default()
                .to_string();
        }
        c
    } else {
        config::Config::new_default()
    };

    let proxy_only = config.proxy_rules.get("/").is_some();
    let logger = Mutex::new(Logger::new(io::stdout()));
    let cache: Mutex<Cache> = Mutex::new(Cache::new(config.cache_ttl as u64));
    let server = Hteapot::new_threaded(config.host.as_str(), config.port, config.threads);
    logger.lock().expect("this doesnt work :C").msg(format!(
        "Server started at http://{}:{}",
        config.host, config.port
    ));
    if config.cache {
        logger
            .lock()
            .expect("this doesnt work :C")
            .msg("Cache Enabled".to_string());
    }
    if proxy_only {
        logger
            .lock()
            .expect("this doesnt work :C")
            .msg("WARNING: All requests are proxied to /. Local paths won’t be used.".to_string());
    }

    server.listen(move |req| {
        // SERVER CORE
        // for each request

        logger.lock().expect("this doesnt work :C").msg(format!(
            "Request {} {}",
            req.method.to_str(),
            req.path
        ));

        let mut full_path = format!("{}{}", config.root, req.path.clone());
        if Path::new(full_path.as_str()).is_dir() {
            let separator = if full_path.ends_with('/') { "" } else { "/" };
            full_path = format!("{}{}{}", full_path, separator, config.index);
        }

        let is_proxy = is_proxy(&config, req.path.clone());
        if proxy_only || is_proxy.is_some() {
            return serve_proxy(is_proxy.unwrap());
        }

        if !Path::new(full_path.as_str()).exists() {
            logger
                .lock()
                .expect("this doesnt work :C")
                .msg(format!("path {} does not exist", req.path));
            return HttpResponse::new(HttpStatus::NotFound, "Not found", None);
        }

        #[cfg(feature = "cgi")]
        {
            let extension = Path::new(&full_path).extension().unwrap();
            let extension = extension.to_str().unwrap();
            println!("File extension: {}", extension);
            let cgi_command = config.cgi_rules.get(extension);
            if cgi_command.is_some() {
                let cgi_command = cgi_command.unwrap();
                logger
                    .lock()
                    .expect("this doesnt work :C")
                    .msg(format!("Runing {} {}", cgi_command, full_path));
                let cgi_result = serve_cgi(cgi_command, &full_path);
                return match cgi_result {
                    Ok(result) => HttpResponse::new(HttpStatus::OK, result, None),
                    Err(_) => HttpResponse::new(
                        HttpStatus::InternalServerError,
                        "Internal server error",
                        None,
                    ),
                };
            }
        }

        let mimetype = get_mime_tipe(&full_path);
        let content: Option<Vec<u8>> = if config.cache {
            let mut cachee = cache.lock().expect("Error locking cache");
            let mut r = cachee.get(req.path.clone());
            if r.is_none() {
                r = serve_file(&full_path);
                if r.is_some() {
                    cachee.set(req.path.clone(), r.clone().unwrap());
                }
            }
            r
        } else {
            serve_file(&full_path)
        };
        match content {
            Some(c) => HttpResponse::new(HttpStatus::OK, c, headers!("Content-Type" => mimetype)),
            None => HttpResponse::new(HttpStatus::NotFound, "Not found", None),
        }
    });
}
