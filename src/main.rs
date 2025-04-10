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
mod handler;
pub mod hteapot;
mod logger;
mod shutdown;
mod utils;

use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;

use std::sync::Mutex;

use cache::Cache;

use hteapot::{Hteapot, HttpRequest, HttpResponse, HttpStatus};

use logger::{LogLevel, Logger};
use std::time::Instant;

use crate::handler::HandlerEngine;
use crate::utils::Context;

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

#[cfg(feature = "cgi")]

fn serve_cgi(
    program: &String,
    path: &String,
    request: HttpRequest,
) -> Result<Vec<u8>, &'static str> {
    use std::{env, io::Write, process::Stdio};
    let query = request
        .args
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&");
    unsafe {
        //TODO: !! fix this, avoid using unsafe , this could conflict simultaneous CGI executions, change to fastCGI ?
        env::set_var("REDIRECT_STATUS", "hteapot");
        env::set_var("SCRIPT_NAME", path);
        env::set_var("SCRIPT_FILENAME", path);
        env::set_var("QUERY_STRING", query);
        env::set_var("REQUEST_METHOD", request.method.to_str());
    }
    let content_type = request.headers.get("CONTENT_TYPE");
    let content_type = match content_type {
        Some(s) => s.clone(),
        None => "".to_string(),
    };
    unsafe {
        //TODO: !! fix this, avoid using unsafe , this could conflict simultaneous CGI executions, change to fastCGI ?
        env::set_var("CONTENT_TYPE", content_type); // Tipo de contenido
        env::set_var("CONTENT_LENGTH", request.body.len().to_string().as_str()); // Longitud del contenido para POST
    }
    let mut child = Command::new(program)
        .arg(&path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn child process");

    let stdin = child.stdin.as_mut().expect("msg");
    stdin
        .write_all(request.body.as_slice())
        .expect("Error writing stdin");
    let output = child.wait_with_output();
    match output {
        Ok(output) => {
            if output.status.success() {
                Ok(output.stdout)
            } else {
                Err("Command exit with non-zero status")
            }
        }
        Err(_) => Err("Error runing command"),
    }
}

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
    let cache: Mutex<Cache<HttpRequest, HttpResponse>> =
        Mutex::new(Cache::new(config.cache_ttl as u64)); // Initialize the cache with TTL

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

    let cache_logger = logger.with_component("cache");
    let http_logger = logger.with_component("http");

    let handlers = HandlerEngine::new();
    // Start listening for HTTP requests
    server.listen(move |req: HttpRequest| {
        // SERVER CORE: For each incoming request, we handle it in this closure
        let start_time = Instant::now(); // Track request processing time
        let req_method = req.method.to_str(); // Get the HTTP method (e.g., GET, POST)

        // Log the incoming request method and path
        http_logger.info(format!("Request {} {}", req_method, req.path));

        if config.cache {
            let cache_start = Instant::now(); // Track cache operation time
            let mut cache_lock = cache.lock().expect("Error locking cache");
            if let Some(response) = cache_lock.get(&req) {
                cache_logger.debug(format!("cache hit for {}", &req.path));
                let elapsed = start_time.elapsed();
                http_logger.debug(format!(
                    "Request processed in {:.6}ms",
                    elapsed.as_secs_f64() * 1000.0 // Log the time taken in milliseconds
                ));
                return Box::new(response);
            } else {
                cache_logger.debug(format!("cache miss for {}", &req.path));
            }
            let cache_elapsed = cache_start.elapsed();
            cache_logger.debug(format!(
                "Cache operation completed in {:.6}µs",
                cache_elapsed.as_micros()
            ));
        }

        let mut ctx = Context {
            request: &req,
            log: &logger,
            config: &config,
            cache: if config.cache {
                Some(&mut cache.lock().unwrap())
            } else {
                None
            },
        };

        let response = handlers.get_handler(&ctx);
        if response.is_none() {
            return HttpResponse::new(HttpStatus::InternalServerError, "content", None);
        }
        let response = response.unwrap().run(&mut ctx);

        // Log how long the request took to process
        let elapsed = start_time.elapsed();
        http_logger.debug(format!(
            "Request processed in {:.6}ms",
            elapsed.as_secs_f64() * 1000.0 // Log the time taken in milliseconds
        ));
        response
        // If content was found, return it with the appropriate headers, otherwise return a 404
    });
}
