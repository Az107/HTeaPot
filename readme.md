# 🍵 HteaPot HTTP Server

[Spanish](docs/readme-es.md) | English

Hteapot is a powerful, Rust-based HTTP server and library designed for high-performance web applications. Effortlessly serve static files and handle HTTP requests with resilience and speed.

# Features

### 1. **Threaded Architecture**
   - Custom thread-based system, capable of handling around **70,000 requests per second**.
   - Emphasizes resilience over peak speed, making it robust under heavy load.

### 2. **Performance Under Load**
   - Steady performance under high concurrency, managing up to **50,000 requests per second** with increased connections.
   - Other's performance degrades significantly under high load, while Hteapot remains stable.

### 3. **Low Error Rate**
   - Achieves a near **100% success rate for 200 OK responses** during stress tests, demonstrating strong resilience.
   - Outperforms others at similar loads, with minimal error rates under extreme concurrency.


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
```Rust
use hteapot::{HttpStatus, Hteapot, HttpRequest};

fn main() {
    let server = Hteapot::new("localhost", 8081);
     teapot.listen(move|req| {
       HttpResponse::new(HttpStatus::IAmATeapot, "Hello i am HTeaPot", None);

     }

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

# Contributing

Contributions are welcome! Feel free to open issues or submit pull requests.

# License

This project is licensed under the MIT License - see the LICENSE file for details.
