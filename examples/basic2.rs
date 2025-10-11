use hteapot::{Hteapot, HttpRequest, HttpResponse, HttpStatus};

fn main() {
    let server = Hteapot::new("localhost", 8081);
    server.listen(move |req: HttpRequest| {
        // This will be executed for each request
        println!(
            "{}",
            String::from_utf8(req.body).unwrap_or("NOPE".to_string())
        );
        HttpResponse::new(HttpStatus::IAmATeapot, "Hello, I am HTeaPot", None)
    });
}
