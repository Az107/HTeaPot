# HteaPot HTTP Server

[Spanish](docs/readme-es.md) | English

HteaPot is a simple HTTP server written in Rust. It allows you to serve static files and handle basic HTTP requests.

# Features

 - Serve static files from a specified root directory
 - Configurable server port and host
 - Basic logging of incoming requests

# Usage

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
# Configuration

You can configure the server using a TOML file. Here's an example configuration:

```toml
[HTEAPOT]
port = 8081 # The port on which the server will listen for incoming connections.
host = "localhost" # The host address to bind the server to. 
root = "public" # The root directory from which to serve files.
```
# Contributing

Contributions are welcome! Feel free to open issues or submit pull requests.

# License

This project is licensed under the MIT License - see the LICENSE file for details.