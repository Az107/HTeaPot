use hteapot::{Hteapot, HttpRequest, HttpResponse, HttpStatus};

fn main() {
    let server = Hteapot::new("localhost", 8081);
    server.listen(move |req: HttpRequest| {
        // This will be executed for each request
        let body = String::from_utf8(req.body).unwrap_or("NOPE".to_string());
        for header in req.headers {
            println!("- {}: {}", header.0, header.1);
        }
        println!("{}", body);
        HttpResponse::new(
            HttpStatus::IAmATeapot,
            format!("Hello, I am HTeaPot\n{}", body),
            None,
        )
    });
}
