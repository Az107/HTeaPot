<h1 align="center">🍵 HTeaPot</h1>
<p align="center"><b>A blazing-fast, minimalist HTTP server library built with Rust</b></p>

<p align="center">
  <a href="https://crates.io/crates/hteapot"><img alt="Crates.io" src="https://img.shields.io/crates/v/hteapot.svg?style=flat-square"></a>
  <a href="https://docs.rs/hteapot"><img alt="Documentation" src="https://img.shields.io/docsrs/hteapot?style=flat-square"></a>
<!--   <a href="https://github.com/Az107/HTeaPot/actions"><img alt="Build Status" src="https://img.shields.io/github/actions/workflow/status/Az107/HTeaPot/rust.yml?branch=main&style=flat-square"></a> -->
  <a href="https://opensource.org/licenses/MIT"><img alt="License: MIT" src="https://img.shields.io/badge/License-MIT-yellow.svg?style=flat-square"></a>
  <a href="https://github.com/Az107/HTeaPot" target="_blank"><img alt="GitHub Repo stars" src="https://img.shields.io/github/stars/Az107/HTeaPot"></a>
</p>

<p align="center">
  <a href="README.md">English</a> |
  <a href="docs/readme_i18n/README_ES.md">Español</a>
</p>


A high-performance, lightweight HTTP server and library built in Rust. HTeaPot is designed to deliver exceptional performance for modern web applications while maintaining a simple and intuitive API.

## 📚 Table of Contents

- [Features](#--features)
- [Getting Started](#-getting-started)
  - [Standalone Server](#standalone-server)
  - [As a Library](#as-a-library)
- [Performance](#-performance)
- [Roadmap](#-roadmap)
- [Contributing](#-contributing)
- [License](#-license)
- [Acknowledgments](#-acknowledgments)

## ✨ Features

###  Exceptional Performance
- **Threaded Architecture**: Powered by a custom-designed thread system that handles **70,000+ requests per second**
- **Consistent Under Load**: Maintains steady performance even under high concurrency scenarios
- **Resilient**: Achieves **near-perfect 100% success rate** for 200 OK responses during stress tests

###  Versatile Functionality
- **Static File Serving**: Efficiently serve static assets with minimal configuration
- **Streaming Support**: Leverages chunked transfer encoding for large files and long-lived connections
- **Flexible API**: Use HTeaPot as a standalone server or as a library in your Rust applications

###  Developer-Friendly
- **Simple Configuration**: Get started quickly with intuitive TOML configuration
- **Extensible Design**: Easily customize behavior for specific use cases
- **Lightweight Footprint**: Zero dependencies and efficient resource usage

## 🚀 Getting Started

### 🔧 Installation

```bash
# Install from crates.io
cargo install hteapot

# Or build from source
git clone https://github.com/Az107/hteapot.git
cd hteapot
cargo build --release
```

### 🖥️ Running the Server

#### Option 1: With Config

1. Create a `config.toml` file:

```toml
[HTEAPOT]
port = 8081        # The port to listen on
host = "localhost" # The host address to bind to
root = "public"    # The root directory to serve files from
```

2. Run the server:

```bash
hteapot ./config.toml
```

#### Option 2: Quick Serve

```bash
hteapot -s ./public/
```

### 🦀 Using as a Library

1. Add HTeaPot to your ```Cargo.toml``` project:

```bash
cargo add hteapot
```

2. Implement in your code: ```example```

```rust
use hteapot::{HttpStatus, HttpResponse, Hteapot, HttpRequest};

fn main() {
    // Create a new server instance
    let server = Hteapot::new("localhost", 8081);
    
    // Define your request handler
    server.listen(move |req: HttpRequest| {
        HttpResponse::new(HttpStatus::IAmATeapot, "Hello, I am HTeaPot", None)
    });
}
```

## 📊 Performance

HTeaPot has been benchmarked against other popular HTTP servers, consistently demonstrating excellent metrics:

```markdown
| Metric        | HTeaPot       | Industry Average       |
|---------------|---------------|------------------------|
| Requests/sec  | 70,000+ req/s | 30,000 - 50,000 req/s  |
| Error rate    | < 0.1%        | 0.5% - 2%              |
| Latency (p99) | 5ms           | 15ms - 30ms            |
| Memory usage  | Low           | Moderate               |
```

##  Roadmap

- [x] HTTP/1.1 support (keep-alive, chunked encoding)
- [x] Library API
- [x] Streaming responses
- [x] Multipart form handling
- [x] Basic routing system
- [ ] HTTPS support
- [ ] Compression (gzip/deflate)
- [ ] WebSocket support
- [ ] Enhanced documentation and examples

##  Contributing

We welcome contributions from the community! To get started:

```sh
# Format the code
cargo fmt

# Lint for warnings
cargo clippy --all-targets --all-features

# Run tests
cargo test
```
See [CONTRIBUTING.md](https://github.com/Az107/HTeaPot/wiki/Contributing) for more details.

##  License

HTeaPot is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

##  Acknowledgments

- The Rust community for their exceptional tools and libraries
- Our contributors who have helped shape this project
- Users who provide valuable feedback and bug reports