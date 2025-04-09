use hteapot::{Hteapot, HttpRequest, HttpResponse, HttpStatus};

fn main() {
    let server = Hteapot::new("localhost", 8081);
    server.listen(move |_req: HttpRequest| {
        // This will be executed for each request
        HttpResponse::new(HttpStatus::IAmATeapot, "Hello, I am HTeaPot", None)
    });
}
