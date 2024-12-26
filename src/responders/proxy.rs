fn serve_proxy(proxy_url: String) -> HttpResponse {
    let raw_response = fetch(&proxy_url);
    match raw_response {
        Ok(raw) => HttpResponse::new_raw(raw),
        Err(_) => HttpResponse::new(HttpStatus::NotFound, "not found", None),
    }
}
