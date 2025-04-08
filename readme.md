# 🍵 HteaPot HTTP Server

[Spanish](docs/i8n/readme-es.md) | English

Hteapot is a powerful, Rust-based HTTP server and library designed for high-performance web applications. Effortlessly serve static files and handle HTTP requests with resilience and speed.

# Features

### 1. **Threaded Architecture**
   Custom thread-based system, capable of handling around **70,000 requests per second**.


### 2. **Performance Under Load**
   Steady performance under high concurrency


### 3. **Low Error Rate**
   - Achieves a near **100% success rate for 200 OK responses** during stress tests, demonstrating strong resilience.
   - Outperforms others at similar loads, with minimal error rates under extreme concurrency.

### 4. **Streaming Support**
  Supports response streaming via chunked transfer encoding, useful for large files or long-lived connections.
### 5. **Library**
  Hteapot can be used as create library , allowing to extend or adapt it to your custom use.


# Use

## standalone http server

You can configure the server using a TOML file. Here's an example configuration:

```toml
[HTEAPOT]
port = 8081 # The port on which the server will listen for incoming connections.
host = "localhost" # The host address to bind the server to.
root = "public" # The root directory from which to serve files.
```

Then running with
```bash
$ hteapot ./config-file.toml
```

or serving a file or folder directly
```bash
$ hteapot -s ./public/
```

## Library

For use hteapot as a library in rust
 1. Install the library
 ```bash
 $ cargo add hteapot
 ```

 2. Then you can use it in your project
```rust
use hteapot::{HttpStatus, HttpResponse, Hteapot, HttpRequest};

fn main() {
    let server = Hteapot::new("localhost", 8081);
    server.listen(move |req: HttpRequest| {
        HttpResponse::new(HttpStatus::IAmATeapot, "Hello, I am HTeaPot", None)
    });
}
```

# Build

1. Clone the repository:
```bash
git clone <repository_url>
```

2. Build the project:
```bash
cargo build --release
```
Run the server with a configuration file:
```bash
Copy code
./target/release/hteapot <config_file_path>
```

# Roadmap

- [x] HTTP/1.1 support (keep-alive, chunked encoding)
- [x] Multipart form handling
- [x] Basic routing
- [x] Library support (use as a crate)
- [x] Streaming responses
- [ ] HTTPS support
- [ ] Compression (gzip/deflate)
- [ ] WebSocket support
- [ ] More modular architecture and documentation

# Contributing

Contributions are welcome! Feel free to open issues or submit pull requests.

[Contributor Guidelines](docs/CONSTRIBUTING.md)

# License

This project is licensed under the MIT License - see the LICENSE file for details.
