use crate::{config::Config, hteapot::HttpRequest, hteapot::HttpResponseCommon};

pub trait Handler {
    fn run(&self, request: &HttpRequest) -> Box<dyn HttpResponseCommon>;
}

pub trait HandlerFactory {
    fn is(config: &Config, request: &HttpRequest) -> Option<Box<dyn Handler>>;
}
