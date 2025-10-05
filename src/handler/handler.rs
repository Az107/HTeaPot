use crate::{config::Config, hteapot::HttpRequest};

pub trait Handler {
    fn is(config: &Config, request: &HttpRequest) -> Option<Box<Self>>;
    fn run(&self, request: HttpRequest) -> Box<HttpRequest>;
}
