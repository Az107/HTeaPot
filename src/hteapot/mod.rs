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
mod http; // HTTP method and status enums
mod request; // Request parsing and builder
mod response; // Response types and streaming
// Status code mapping

// use std::sync::atomic::{AtomicBool, Ordering};

use std::time::Duration;

// Public API exposed by this module
pub use self::request::HttpRequest;
pub use engine::Hteapot;
pub use http::HttpHeaders;
pub use http::HttpMethod;
pub use http::HttpStatus;

pub use response::{
    BufferedResponse, HttpResponse, HttpResponseCommon, StreamedResponse, TunnelResponse,
};

/// Crate version as set by `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Size of the buffer used for reading from the TCP stream.
const BUFFER_SIZE: usize = 1024 * 2;

/// Time-to-live for keep-alive connections.
const KEEP_ALIVE_TTL: Duration = Duration::from_secs(10);

#[cfg(test)]
mod tests {
    use crate::{HttpResponse, HttpStatus};
    use http::HttpHeaders;
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    use super::*;

    #[test]
    fn test_http_response_maker() {
        let mut response = HttpResponse::new(HttpStatus::IAmATeapot, "Hello, World!", None);
        let response = String::from_utf8(response.to_bytes()).unwrap();
        let expected_response = format!(
            "HTTP/1.1 418 I'm a teapot\r\nContent-Length: 13\r\nServer: HTeaPot/{}\r\n\r\nHello, World!\r\n",
            VERSION //TODO: fix
        );
        let expected_response_list = expected_response.split("\r\n");
        for item in expected_response_list {
            assert!(response.contains(item));
        }
    }

    #[test]
    fn test_keep_alive_connection() {
        let mut headers = HttpHeaders::new();
        headers.insert("Connection", "keep-alive");
        headers.insert("Content-Length", "15");
        headers.insert(
            "Keep-Alive",
            &format!("timeout={}", KEEP_ALIVE_TTL.as_secs()),
        );

        let mut response = HttpResponse::new(HttpStatus::OK, "Keep-Alive Test", Some(headers));

        let response_bytes = response.to_bytes();
        let response_str = String::from_utf8(response_bytes.clone()).unwrap();

        assert!(response_str.contains("HTTP/1.1 200 OK"));
        assert!(response_str.contains("Content-Length: 15"));
        assert!(response_str.contains("Connection: keep-alive"));
        assert!(response_str.contains("Keep-Alive: timeout=10"));
        assert!(response_str.contains("Server: HTeaPot/"));
        assert!(response_str.contains("Keep-Alive Test"));
        let mut headers = HttpHeaders::new();
        headers.insert("Connection", "keep-alive");
        headers.insert("Content-Length", "14");
        headers.insert(
            "Keep-Alive",
            &format!("timeout={}", KEEP_ALIVE_TTL.as_secs()),
        );

        let mut second_response =
            HttpResponse::new(HttpStatus::OK, "Second Request", Some(headers));

        let second_response_bytes = second_response.to_bytes();
        let second_response_str = String::from_utf8(second_response_bytes.clone()).unwrap();

        assert!(second_response_str.contains("HTTP/1.1 200 OK"));
        assert!(second_response_str.contains("Content-Length: 14"));
        assert!(response_str.contains("Connection: keep-alive"));
        assert!(response_str.contains("Keep-Alive: timeout=10"));
        assert!(response_str.contains("Server: HTeaPot/"));
        assert!(second_response_str.contains("Second Request"));
    }
}
