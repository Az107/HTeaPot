use crate::{hteapot::HttpResponseCommon, utils::Context};

pub trait Handler {
    fn run(&self, context: &mut Context) -> Box<dyn HttpResponseCommon>;
}

pub trait HandlerFactory {
    fn is(context: &Context) -> Option<Box<dyn Handler>>;
}
