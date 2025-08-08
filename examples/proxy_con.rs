use hteapot::{Hteapot, HttpMethod, HttpRequest, HttpResponse, TunnelResponse, headers};

fn main() {
    let server = Hteapot::new("0.0.0.0", 8081);
    server.listen(move |req: HttpRequest| {
        println!("New request to {} {}!", req.method.to_str(), &req.path);
        if req.method == HttpMethod::CONNECT {
            TunnelResponse::new(&req.path)
        } else {
            let secure_path = req.path.replace("http", "https");
            HttpResponse::new(
                hteapot::HttpStatus::MovedPermanently,
                "Moved",
                headers! {"location" => secure_path},
            )
        }
    });
}
