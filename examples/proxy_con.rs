use hteapot::{Hteapot, HttpMethod, HttpRequest, HttpResponse, TunnelResponse};

fn main() {
    let server = Hteapot::new_threaded("0.0.0.0", 8081, 4);
    server.listen(move |req: HttpRequest| {
        println!("New request to {} {}!", req.method.to_str(), &req.path);
        if req.method == HttpMethod::CONNECT {
            TunnelResponse::new(&req.path)
        } else {
            let addr = if let Some(addr) = req.headers.get("host") {
                addr
            } else {
                println!("Error: getting host");
                return HttpResponse::new(
                    hteapot::HttpStatus::InternalServerError,
                    "content",
                    None,
                );
            };
            let resp = req.brew(addr);
            if resp.is_err() {
                println!("erro: {:?}", resp.err());
                HttpResponse::new(hteapot::HttpStatus::InternalServerError, "content", None)
            } else {
                resp.unwrap()
            }
        }
    });
}
