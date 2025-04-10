mod cache;
mod config;
pub mod hteapot;
mod logger;
mod utils;

use std::{fs, io};

use std::path::Path;

use std::sync::Mutex;

use cache::Cache;
use config::Config;
use hteapot::{Hteapot, HttpRequest, HttpResponse, HttpStatus};
use utils::get_mime_tipe;

use logger::{Logger, LogLevel};
use std::time::Instant;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn is_proxy(config: &Config, req: HttpRequest) -> Option<(String, HttpRequest)> {
    for proxy_path in config.proxy_rules.keys() {
        let path_match = req.path.strip_prefix(proxy_path);
        if path_match.is_some() {
            let new_path = path_match.unwrap();
            let url = config.proxy_rules.get(proxy_path).unwrap().clone();
            let mut proxy_req = req.clone();
            proxy_req.path = new_path.to_string();
            proxy_req.headers.remove("Host");
            let host_parts: Vec<_> = url.split("://").collect();
            let host = if host_parts.len() == 1 {
                host_parts.first().unwrap()
            } else {
                host_parts.last().clone().unwrap()
            };
            proxy_req.header("Host", host);
            return Some((url, proxy_req));
        }
    }
    None
}

fn serve_file(path: &String) -> Option<Vec<u8>> {
    let r = fs::read(path);
    if r.is_ok() { Some(r.unwrap()) } else { None }
}

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() == 1 {
        println!("Hteapot {}", VERSION);
        println!("usage: {} <config file>", args[0]);
        return;
    }
    let config = match args[1].as_str() {
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
            let mut c = config::Config::new_default();
            let serving_path = Some(args.get(2).unwrap().clone());
            let serving_path_str = serving_path.unwrap();
            let serving_path_str = serving_path_str.as_str();
            let serving_path = Path::new(serving_path_str);
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
            c.host = "0.0.0.0".to_string();
            c
        }
        _ => config::Config::load_config(&args[1]),
    };

    let proxy_only = config.proxy_rules.get("/").is_some();
    let logger = match config.log_file.clone() {
        Some(file_name) => {
            let file = fs::File::create(file_name.clone());
            match file {
                Ok(file) => Logger::new(file, LogLevel::INFO, "main"),
                Err(e) => {
                    println!("Failed to create log file: {:?}. Using stdout instead.", e);
                    Logger::new(io::stdout(), LogLevel::INFO, "main")
                }
            }
        }
        None => Logger::new(io::stdout(), LogLevel::INFO, "main"),
    };

    let cache: Mutex<Cache> = Mutex::new(Cache::new(config.cache_ttl as u64));
    let server = Hteapot::new_threaded(config.host.as_str(), config.port, config.threads);
    logger.info(format!(
        "Server started at http://{}:{}",
        config.host, config.port
    ));
    if config.cache {
        logger.info("Cache Enabled".to_string());
    }
    if proxy_only {
        logger
            .warn("WARNING: All requests are proxied to /. Local paths won’t be used.".to_string());
    }

    // Create component loggers
    let proxy_logger = logger.with_component("proxy");
    let cache_logger = logger.with_component("cache");
    let http_logger = logger.with_component("http");


    server.listen(move |req| {
        // SERVER CORE
        // for each request
        let start_time = Instant::now();
        let req_method = req.method.to_str();
        let req_path = req.path.clone();


        http_logger.info(format!("Request {} {}", req.method.to_str(), req.path));

        let is_proxy = is_proxy(&config, req.clone());

        if proxy_only || is_proxy.is_some() {
            let (host, proxy_req) = is_proxy.unwrap();
            proxy_logger.info(format!(
                "Proxying request {} {} to {}",
                req_method, req_path, host
            ));
            let res = proxy_req.brew(host.as_str());
            let elapsed = start_time.elapsed();
            if res.is_ok() {
                let response = res.unwrap();
                proxy_logger.info(format!(
                    "Proxy request processed in {:.6}ms",
                    elapsed.as_secs_f64() * 1000.0
                ));
                return response;
            } else {
                proxy_logger.error(format!("Proxy request failed: {:?}", res.err()));
                return HttpResponse::new(
                    HttpStatus::InternalServerError,
                    "Internal Server Error",
                    None,
                );
            }
        }

        let mut full_path = format!("{}{}", config.root, req.path.clone());
        if Path::new(full_path.as_str()).is_dir() {
            let separator = if full_path.ends_with('/') { "" } else { "/" };
            full_path = format!("{}{}{}", full_path, separator, config.index);
        }

        if !Path::new(full_path.as_str()).exists() {
            http_logger.warn(format!("Path {} does not exist", req.path));
            return HttpResponse::new(HttpStatus::NotFound, "Not found", None);
        }
        let mimetype = get_mime_tipe(&full_path);
        let content: Option<Vec<u8>> = if config.cache {
            let mut cachee = cache.lock().expect("Error locking cache");
            let cache_start = Instant::now();
            let mut r = cachee.get(req.path.clone());
            if r.is_none() {
                cache_logger.debug(format!("cache miss for {}", req.path));
                r = serve_file(&full_path);
                if r.is_some() {
                    cache_logger.info(format!("Adding {} to cache", req.path));
                    cachee.set(req.path.clone(), r.clone().unwrap());
                }
            } else {
                cache_logger.debug(format!("cache hit for {}", req.path));
            }

            let cache_elapsed = cache_start.elapsed();
            cache_logger.debug(format!(
                "Cache operation completed in {:.6}µs", 
                cache_elapsed.as_micros()
            ));
            r
        } else {
            serve_file(&full_path)
        };

        let elapsed = start_time.elapsed();
        http_logger.info(format!(
            "Request processed in {:.6}ms",
            elapsed.as_secs_f64() * 1000.0
        ));
        match content {
            Some(c) => HttpResponse::new(HttpStatus::OK, c, headers!("Content-Type" => mimetype)),
            None => HttpResponse::new(HttpStatus::NotFound, "Not found", None),
        }
    });
}
