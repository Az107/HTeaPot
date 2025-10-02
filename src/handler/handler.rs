use crate::{config::Config, hteapot::HttpRequest};

pub trait Handler {
    fn is(config: Config, request: HttpRequest) -> bool;
    fn run(request: HttpRequest) -> Box<HttpRequest>;
}
