use hteapot::HttpRequest;

use crate::{config::Config, hteapot::HttpResponse, logger::Logger};

trait Serve {
    fn check(&self) -> bool;
    fn serve(&self) -> HttpResponse;
}

struct Context {
    request: HttpRequest,
    logger: Logger<t>,
    conf: Config,
}
