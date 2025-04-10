mod cache;
mod config;
pub mod hteapot;
mod logger;
mod utils;

use std::{fs, io, path::PathBuf};
use std::path::Path;
use std::sync::Mutex;

use cache::Cache;
use config::Config;
use hteapot::{Hteapot, HttpRequest, HttpResponse, HttpStatus};
use utils::get_mime_tipe;

use logger::{Logger, LogLevel};
use std::time::Instant;

const VERSION: &str = env!("CARGO_PKG_VERSION");

// Safely join paths and ensure the result is within the root directory
// Try to canonicalize to resolve any '..' components
// Ensure the canonicalized path is still within the root directory
// Check if the path exists before canonicalizing
fn safe_join_paths(root: &str, requested_path: &str) -> Option<PathBuf> {
    let root_path = Path::new(root).canonicalize().ok()?;
    let requested_full_path = root_path.join(requested_path.trim_start_matches("/"));
    
    if !requested_full_path.exists() {
        return None;
    }
    

    let canonical_path = requested_full_path.canonicalize().ok()?;
    
    if canonical_path.starts_with(&root_path) {
        Some(canonical_path)
    } else {
        None
    }
}

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

// Change from &string to &PathBuf cos PathBuf explicitly represents a file system path as an owned buffer,
// making it clear that the data is intended to be a path rather than just any string. 
// This reduces errors by enforcing the correct type for file system operations.
fn serve_file(path: &PathBuf) -> Option<Vec<u8>> {
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
            .warn("WARNING: All requests are proxied to /. Local paths won't be used.".to_string());
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

        // Safely resolve the requested path
        let safe_path_result = if req.path == "/" {
            // Handle root path specially
            let root_path = Path::new(&config.root).canonicalize();
            if root_path.is_ok() {
                let index_path = root_path.unwrap().join(&config.index);
                if index_path.exists() {
                    Some(index_path)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            safe_join_paths(&config.root, &req.path)
        };

        // Handle directory paths
        let safe_path = match safe_path_result {
            Some(path) => {
                if path.is_dir() {
                    let index_path = path.join(&config.index);
                    if index_path.exists() {
                        index_path
                    } else {
                        http_logger.warn(format!("Index file not found in directory: {}", req.path));
                        return HttpResponse::new(HttpStatus::NotFound, "Index not found", None);
                    }
                } else {
                    path
                }
            },
            None => {
                http_logger.warn(format!("Path not found or access denied: {}", req.path));
                return HttpResponse::new(HttpStatus::NotFound, "Not found", None);
            }
        };

        let mimetype = get_mime_tipe(&safe_path.to_string_lossy().to_string());
        let content: Option<Vec<u8>> = if config.cache {
            let mut cachee = cache.lock().expect("Error locking cache");
            let cache_start = Instant::now();
            let cache_key = req.path.clone();
            let mut r = cachee.get(cache_key.clone());
            if r.is_none() {
                cache_logger.debug(format!("cache miss for {}", cache_key));
                r = serve_file(&safe_path);
                if r.is_some() {
                    cache_logger.info(format!("Adding {} to cache", cache_key));
                    cachee.set(cache_key, r.clone().unwrap());
                }
            } else {
                cache_logger.debug(format!("cache hit for {}", cache_key));
            }

            let cache_elapsed = cache_start.elapsed();
            cache_logger.debug(format!(
                "Cache operation completed in {:.6}µs", 
                cache_elapsed.as_micros()
            ));
            r
        } else {
            serve_file(&safe_path)
        };

        let elapsed = start_time.elapsed();
        http_logger.info(format!(
            "Request processed in {:.6}ms",
            elapsed.as_secs_f64() * 1000.0
        ));
        
        match content {
            Some(c) => {
                let mut headers = headers!("Content-Type" => mimetype);
                if let Some(ref mut map) = headers {  // Unwrap the Option safely
                    map.insert("X-Content-Type-Options".to_string(), "nosniff".to_string());
                }
                HttpResponse::new(HttpStatus::OK, c, headers)  // Pass the Option directly
            },
            None => HttpResponse::new(HttpStatus::NotFound, "Not found", None),
        }
    });
}