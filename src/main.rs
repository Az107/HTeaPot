//! # ☕ Hteapot Web Server
//!
//! **Hteapot** is a fast, lightweight, and highly extensible HTTP server written in idiomatic Rust.
//! Designed for simplicity and performance, it supports:
//!
//! - 🔁 **Reverse Proxying** — Forward requests to other servers based on custom routing rules
//! - 📁 **Static File Serving** — Serve local files from a configurable directory
//! - ⚡ **In-Memory Caching** — Reduce disk I/O with optional response caching
//! - 📜 **Structured Logging** — Toggle between file or console logging with fine-grained log levels
//! - 🧵 **Multithreading** — Handle requests concurrently using a configurable thread pool
//!
//! ## Use Cases
//!
//! - Local development server
//! - Lightweight reverse proxy
//! - Static site deployment
//! - Embedded use in tools or microservices
//!
//! ## Entry Point
//!
//! This crate's primary entry point is the `main.rs` module. It sets up configuration,
//! logging, caching, and request routing via the [`Hteapot`](crate::hteapot::Hteapot) engine.
//!
//! ## Example
//!
//! ```sh
//! $ hteapot ./config.toml
//! ```
//!
//! Or serve a single file quickly:
//!
//! ```sh
//! $ hteapot --serve ./index.html
//! ```
//!
//! See the [`config`](crate::config) module for configuration options and structure.
mod cache;
mod config;
pub mod hteapot;
mod http_responders;
mod logger;
mod shutdown;
mod utils;

use std::fs;
use std::io;
use std::path::Path;
use std::sync::Mutex;

use cache::Cache;
use hteapot::HttpMethod;
use hteapot::TunnelResponse;
use hteapot::{Hteapot, HttpRequest, HttpResponse, HttpStatus};
use utils::get_mime_tipe;

use logger::{LogLevel, Logger};
use std::time::Instant;

use http_responders::file::{safe_join_paths, serve_file};
use http_responders::proxy::is_proxy;

/// Main entry point of the Hteapot server.
///
/// Handles command-line interface, config file parsing, optional file-serving mode,
/// logger initialization, and server startup. Incoming requests are processed via
/// proxy rules or served from local files with optional caching.
///
/// # CLI Usage
/// - `hteapot config.toml` – Start with a full configuration file.
/// - `hteapot --serve ./file.html` – Serve a single file.
/// - `hteapot --help` or `--version` – Show usage info.
///
/// This function initializes core components:
/// - Configuration (`Config`)
/// - Logging (`Logger`)
/// - Optional response caching
/// - HTTP server via [`Hteapot::new_threaded`](crate::hteapot::Hteapot::new_threaded)

fn main() {
    // Parse CLI args and handle --help / --version / --serve flags
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() == 1 {
        println!("Hteapot {}", hteapot::VERSION);
        println!("usage: {} <config file>", args[0]);
        return;
    }

    // Initialize logger based on config or default to stdout
    let mut config = match args[1].as_str() {
        "--help" | "-h" => {
            println!("Hteapot {}", hteapot::VERSION);
            println!("usage: {} <config file>", args[0]);
            return;
        }
        "--version" | "-v" => {
            println!("Hteapot {}", hteapot::VERSION);
            return;
        }
        "--serve" | "-s" => {
            let path = args.get(2).unwrap().clone();
            config::Config::new_serve(&path)
        }
        "--proxy" => {
            let c = config::Config::new_proxy();
            c
        }
        _ => config::Config::load_config(&args[1]),
    };

    if args.contains(&"-p".to_string()) {
        let i = args.iter().position(|e| *e == "-p".to_string()).unwrap();
        let port = args[i + 1].clone();
        let port = port.parse::<u16>();
        if port.is_err() {
            println!("Invalid port provided");
            return;
        }
        let port = port.unwrap();
        config.port = port;
    }

    // Determine if the server should proxy all requests
    let proxy_only = config.proxy_rules.get("/").is_some();

    let min_log = if cfg!(debug_assertions) {
        LogLevel::DEBUG
    } else {
        LogLevel::INFO
    };
    // Initialize the logger based on the config or default to stdout if the log file can't be created
    let logger = match config.log_file.clone() {
        Some(file_name) => {
            let file = fs::File::create(file_name.clone()); // Attempt to create the log file
            match file {
                // If creating the file fails, log to stdout instead
                Ok(file) => Logger::new(file, min_log, "main"), // If successful, use the file
                Err(e) => {
                    println!("Failed to create log file: {:?}. Using stdout instead.", e);
                    Logger::new(io::stdout(), min_log, "main") // Log to stdout
                }
            }
        }
        None => Logger::new(io::stdout(), min_log, "main"), // If no log file is specified, use stdout
    };

    // Set up the cache with thread-safe locking
    // The Mutex ensures that only one thread can access the cache at a time,
    // preventing race conditions when reading and writing to the cache.
    let cache: Mutex<Cache> = Mutex::new(Cache::new(config.cache_ttl as u64)); // Initialize the cache with TTL

    // Create a new threaded HTTP server with the provided host, port, and number of threads
    let mut server = Hteapot::new_threaded(config.host.as_str(), config.port, config.threads);

    //Configure graceful shutdown from ctrl+c
    shutdown::setup_graceful_shutdown(&mut server, logger.clone());

    logger.info(format!(
        "Server started at http://{}:{}",
        config.host, config.port
    )); // Log that the server has started

    // Log whether the cache is enabled based on the config setting
    if config.cache {
        logger.info("Cache Enabled".to_string());
    }

    // If proxy-only mode is enabled, issue a warning that local paths won't be used
    if proxy_only {
        logger
            .warn("WARNING: All requests are proxied to /. Local paths won't be used.".to_string());
    }

    // Create separate loggers for each component (proxy, cache, and HTTP)
    // This allows for more granular control over logging and better separation of concerns
    let proxy_logger = logger.with_component("proxy");
    let cache_logger = logger.with_component("cache");
    let http_logger = logger.with_component("http");

    // Start listening for HTTP requests
    server.listen(move |req: HttpRequest| {
        // SERVER CORE: For each incoming request, we handle it in this closure
        let start_time = Instant::now(); // Track request processing time
        let req_method = req.method.to_str(); // Get the HTTP method (e.g., GET, POST)
        let req_path = req.path.clone(); // Get the requested path

        // Log the incoming request method and path
        http_logger.info(format!("Request {} {}", req_method, req.path));

        if proxy_only && req.method == HttpMethod::CONNECT {
            return TunnelResponse::new(&req.path);
        }
        // Check if the request should be proxied (either because proxy-only mode is on, or it matches a rule)
        let is_proxy = is_proxy(&config, req.clone() as HttpRequest);
        if proxy_only || is_proxy.is_some() {
            // ⚠️ TODO: refactor proxy handling
            // If proxying is enabled or this request matches a proxy rule, handle it
            if req.method == hteapot::HttpMethod::CONNECT {
                return TunnelResponse::new(&req.path);
            }
            if is_proxy.is_none() {
                return HttpResponse::new(HttpStatus::NotAcceptable, "", None);
            }
            let (host, proxy_req) = is_proxy.unwrap();
            // Get the target host and modified request
            proxy_logger.info(format!(
                "Proxying request {} {} to {}",
                req_method, req_path, host
            ));

            // Perform the proxy request (forward the request to the target server)
            let res = proxy_req.brew(host.as_str());
            let elapsed = start_time.elapsed(); // Measure the time taken to process the proxy request
            if res.is_ok() {
                // If the proxy request is successful, log the time taken and return the response
                let response = res.unwrap();
                proxy_logger.info(format!(
                    "Proxy request processed in {:.6}ms",
                    elapsed.as_secs_f64() * 1000.0 // Log the time taken in milliseconds
                ));
                return response;
            } else {
                // If the proxy request fails, log the error and return a 500 Internal Server Error
                proxy_logger.error(format!("Proxy request failed: {:?}", res.err()));
                return HttpResponse::new(
                    HttpStatus::InternalServerError,
                    "Internal Server Error",
                    None,
                );
            }
        }

        // If the request is not a proxy request, resolve the requested path safely
        let safe_path_result = if req.path == "/" {
            // Special handling for the root "/" path
            let root_path = Path::new(&config.root).canonicalize();
            if root_path.is_ok() {
                // If the root path exists and is valid, try to join the index file
                let index_path = root_path.unwrap().join(&config.index);
                if index_path.exists() {
                    Some(index_path) // If index exists, return its path
                } else {
                    None // If no index exists, return None
                }
            } else {
                None // If the root path is invalid, return None
            }
        } else {
            // For any other path, resolve it safely using the `safe_join_paths` function
            safe_join_paths(&config.root, &req.path)
        };

        // Handle the case where the resolved path is a directory
        let safe_path = match safe_path_result {
            Some(path) => {
                if path.is_dir() {
                    // If it's a directory, check for the index file in that directory
                    let index_path = path.join(&config.index);
                    if index_path.exists() {
                        index_path // If index exists, return its path
                    } else {
                        // If no index file exists, log a warning and return a 404 response
                        http_logger
                            .warn(format!("Index file not found in directory: {}", req.path));
                        return HttpResponse::new(HttpStatus::NotFound, "Index not found", None);
                    }
                } else {
                    path // If it's not a directory, just return the path
                }
            }
            None => {
                // If the path is invalid or access is denied, return a 404 response
                http_logger.warn(format!("Path not found or access denied: {}", req.path));
                return HttpResponse::new(HttpStatus::NotFound, "Not found", None);
            }
        };

        // Determine the MIME type for the file based on its extension
        let mimetype = get_mime_tipe(&safe_path.to_string_lossy().to_string());

        // Try to serve the file from the cache, or read it from disk if not cached
        let content: Option<Vec<u8>> = if config.cache {
            // Lock the cache to ensure thread-safe access
            let mut cachee = cache.lock().expect("Error locking cache");
            let cache_start = Instant::now(); // Track cache operation time
            let cache_key = req.path.clone(); // Use the request path as the cache key
            let mut r = cachee.get(cache_key.clone()); // Try to get the content from cache
            if r.is_none() {
                // If cache miss, read the file from disk and store it in cache
                cache_logger.debug(format!("cache miss for {}", cache_key));
                r = serve_file(&safe_path);
                if r.is_some() {
                    // If the file is read successfully, add it to the cache
                    cache_logger.debug(format!("Adding {} to cache", cache_key));
                    cachee.set(cache_key, r.clone().unwrap());
                }
            } else {
                // If cache hit, log it
                cache_logger.debug(format!("cache hit for {}", cache_key));
            }

            // Log how long the cache operation took
            let cache_elapsed = cache_start.elapsed();
            cache_logger.debug(format!(
                "Cache operation completed in {:.6}µs",
                cache_elapsed.as_micros()
            ));
            r // Return the cached content (or None if not found)
        } else {
            // If cache is disabled, read the file from disk
            serve_file(&safe_path)
        };

        // Log how long the request took to process
        let elapsed = start_time.elapsed();
        http_logger.debug(format!(
            "Request processed in {:.6}ms",
            elapsed.as_secs_f64() * 1000.0 // Log the time taken in milliseconds
        ));

        // If content was found, return it with the appropriate headers, otherwise return a 404
        match content {
            Some(c) => {
                // If content is found, create response with proper headers and a 200 OK status
                let headers = headers!(
                    "Content-Type" => mimetype,
                    "X-Content-Type-Options" => "nosniff"
                );
                HttpResponse::new(HttpStatus::OK, c, headers)
            }
            None => {
                // If no content is found, return a 404 Not Found response
                HttpResponse::new(HttpStatus::NotFound, "Not found", None)
            }
        }
    });
}
