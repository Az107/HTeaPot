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
mod logger;
mod utils;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use std::path::Path;
use std::sync::Mutex;
use std::{fs, io, path::PathBuf};

use cache::Cache;
use config::Config;
use hteapot::{Hteapot, HttpRequest, HttpResponse, HttpStatus};
use utils::get_mime_tipe;

use logger::{LogLevel, Logger};
use std::time::Instant;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Attempts to safely join a root directory and a requested relative path.
///
/// Ensures that the resulting path:
/// - Resolves symbolic links and `..` segments via `canonicalize`
/// - Remains within the bounds of the specified root directory
/// - Actually exists on disk
///
/// This protects against directory traversal vulnerabilities, such as accessing
/// files outside of the intended root (e.g., `/etc/passwd`).
///
/// # Arguments
/// * `root` - The root directory from which serving is allowed.
/// * `requested_path` - The path requested by the client (usually from the URL).
///
/// # Returns
/// `Some(PathBuf)` if the resolved path exists and is within the root. `None` otherwise.
///
/// # Example
/// ```
/// let safe_path = safe_join_paths("/var/www", "/index.html");
/// assert!(safe_path.unwrap().ends_with("index.html"));
/// ```
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

/// Determines whether a given HTTP request should be proxied based on the configuration.
///
/// If a matching proxy rule is found in `config.proxy_rules`, the function rewrites the
/// request path and updates the `Host` header accordingly.
///
/// # Arguments
/// * `config` - Server configuration containing proxy rules.
/// * `req` - The original HTTP request.
///
/// # Returns
/// `Some((proxy_url, modified_request))` if the request should be proxied, otherwise `None`.
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

/// Reads the content of a file from the filesystem.
///
/// # Arguments
/// * `path` - A reference to a `PathBuf` representing the target file.
///
/// # Returns
/// `Some(Vec<u8>)` if the file is read successfully, or `None` if an error occurs.
///
/// # Notes
/// Uses `PathBuf` instead of `&str` to clearly express intent and reduce path handling bugs.
///
/// # See Also
/// [`std::fs::read`](https://doc.rust-lang.org/std/fs/fn.read.html)
fn serve_file(path: &PathBuf) -> Option<Vec<u8>> {
    let r = fs::read(path);
    if r.is_ok() { Some(r.unwrap()) } else { None }
}
//
// Suggest to use .ok()? instead of manual unwrap/if is_ok for more idiomatic error handling:
// fn serve_file(path: &PathBuf) -> Option<Vec<u8>> {
// fs::read(path).ok()
// }
//
//

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
        println!("Hteapot {}", VERSION);
        println!("usage: {} <config file>", args[0]);
        return;
    }

    // Initialize logger based on config or default to stdout
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

    // Determine if the server should proxy all requests
    let proxy_only = config.proxy_rules.get("/").is_some();

    // Initialize the logger based on the config or default to stdout if the log file can't be created
    let logger = match config.log_file.clone() {
        Some(file_name) => {
            let file = fs::File::create(file_name.clone()); // Attempt to create the log file
            match file {
                // If creating the file fails, log to stdout instead
                Ok(file) => Logger::new(file, LogLevel::INFO, "main"), // If successful, use the file
                Err(e) => {
                    println!("Failed to create log file: {:?}. Using stdout instead.", e);
                    Logger::new(io::stdout(), LogLevel::INFO, "main") // Log to stdout
                }
            }
        }
        None => Logger::new(io::stdout(), LogLevel::INFO, "main"), // If no log file is specified, use stdout
    };

    // Set up the cache with thread-safe locking
    // The Mutex ensures that only one thread can access the cache at a time,
    // preventing race conditions when reading and writing to the cache.
    let cache: Mutex<Cache> = Mutex::new(Cache::new(config.cache_ttl as u64)); // Initialize the cache with TTL

    // Create a new threaded HTTP server with the provided host, port, and number of threads
    let server = Hteapot::new_threaded(config.host.as_str(), config.port, config.threads);

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
    server.listen(move |req| {
        // SERVER CORE: For each incoming request, we handle it in this closure
        let start_time = Instant::now(); // Track request processing time
        let req_method = req.method.to_str(); // Get the HTTP method (e.g., GET, POST)
        let req_path = req.path.clone(); // Get the requested path

        // Log the incoming request method and path
        http_logger.info(format!("Request {} {}", req_method, req.path));

        // Check if the request should be proxied (either because proxy-only mode is on, or it matches a rule)
        let is_proxy = is_proxy(&config, req.clone());
        if proxy_only || is_proxy.is_some() {
            // If proxying is enabled or this request matches a proxy rule, handle it
            let (host, proxy_req) = is_proxy.unwrap(); // Get the target host and modified request
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
                    cache_logger.info(format!("Adding {} to cache", cache_key));
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
        http_logger.info(format!(
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
