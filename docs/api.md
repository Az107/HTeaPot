# API

HteaPot can be used as a library as well.
It intends to be easy to setup but powerfull in what matters

---

##  ✨ Basic Example
```rust
use hteapot::{HttpStatus, HttpResponse, Hteapot, HttpRequest};

fn main() {
    let server = Hteapot::new("localhost", 8081);
    server.listen(move |req: HttpRequest| {
        // This will be executed for each request
        HttpResponse::new(HttpStatus::IAmATeapot, "Hello, I am HTeaPot", None)
    });
}
```
This will start a server on localhost:8081, and return a static response to any incoming request.

**Important:** the `listen` method will block the execution, a method to gracefull stop its planed

## Streaming Responses
HTeaPot also supports streaming responses by using the StreamedResponse type.

```rust
fn main() {
    let server = Hteapot::new("localhost", 8081);
    server.listen(move |req: HttpRequest| {
        let times = 5;
        let response = StreamedResponse::new(move |sender| {
            for i in 0..times {
                let data = format!("{i}-abcd\n").into_bytes();
                let _ = sender.send(data);
                thread::sleep(Duration::from_secs(1));
            }
        });
    });
}
```

This sends a chunk of data every second for 5 seconds.
Each call to sender.send(...) pushes data directly to the client.

---

## 📘 API Reference

### Hteapot::new(host: &str, port: u16)
Creates a new HTTP server bound to the given host and port.

### Hteapot::listen(handler: impl Fn(HttpRequest) -> impl Into<HttpResponseType>)
Starts the HTTP server and handles all incoming requests with the given closure.

The closure receives an HttpRequest and must return either:
- HttpResponse for static responses
- StreamedResponse for streamed output

### HttpResponse::new(status: HttpStatus, body: impl Into<Vec<u8>>, headers: Option<Headers>)

Creates a static response with the given status code, body, and optional headers.

### StreamedResponse::new(callback: impl Fn(Sender))

Creates a streaming response. The callback is executed when the connection opens, and can use the provided Sender to send chunks of data.
