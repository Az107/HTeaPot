use hteapot::{Hteapot, HttpMethod, HttpRequest, HttpResponse, TunnelResponse};

fn main() {
    let server = Hteapot::new_threaded("0.0.0.0", 8081, 3);
    server.listen(move |req: HttpRequest| {
        println!("New request to {} {}!", req.method.to_str(), &req.path);
        if req.method == HttpMethod::CONNECT {
            TunnelResponse::new(&req.path)
        } else {
            println!("{:?}", req);
            let addr = req.headers.get("host");
            let addr = if let Some(addr) = addr {
                addr
            } else {
                if let Some(addr) = req.headers.get("Host") {
                    addr
                } else {
                    return HttpResponse::new(
                        hteapot::HttpStatus::InternalServerError,
                        "content",
                        None,
                    );
                }
            };
            req.brew(addr).unwrap_or(HttpResponse::new(
                hteapot::HttpStatus::InternalServerError,
                "content",
                None,
            ))
        }
    });
}
