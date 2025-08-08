// Written by Alberto Ruiz 2024-03-08
//
// This is the HTTP server module, it will handle the requests and responses
// Also provides utilities to parse the requests and build the response

//! HTeaPot HTTP server core.
//!
//! This module provides a multithreaded HTTP/1.1 server built for performance and ease of use.
//! It handles request parsing, response building, connection lifecycle (keep-alive)
//! and hooks.
//!
//! Core types:
//! - [`Hteapot`] — the main server entry point
//! - [`HttpRequest`] and [`HttpResponse`] — re-exported from submodules
//!
//! Use [`Hteapot::listen`] to start a server with a request handler closure.
//! ```

/// Submodules for HTTP functionality.
pub mod brew; // HTTP client implementation
mod engine;
mod methods; // HTTP method and status enums
mod request; // Request parsing and builder
mod response; // Response types and streaming
mod status; // Status code mapping

// use std::sync::atomic::{AtomicBool, Ordering};

// Public API exposed by this module
pub use self::request::HttpRequest;
pub use engine::Hteapot;
pub use methods::HttpMethod;

pub use response::{HttpResponse, StreamedResponse, TunnelResponse};
pub use status::HttpStatus;

/// Crate version as set by `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Size of the buffer used for reading from the TCP stream.
const BUFFER_SIZE: usize = 1024 * 2;
