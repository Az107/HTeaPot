use crate::{
    config::Config,
    handler::handler::{Handler, HandlerFactory},
    hteapot::HttpRequest,
};

pub mod file;
mod handler;
pub mod proxy;

/// Type alias for a handler factory function.
///
/// A factory takes a reference to the current `Config` and `HttpRequest`
/// and returns an `Option<Box<dyn Handler>>`. It returns `Some(handler)`
/// if it can handle the request, or `None` if it cannot.
type Factory = fn(&Config, &HttpRequest) -> Option<Box<dyn Handler>>;

/// List of all available handler factories.
///
/// New handlers can be added to this array to make them available
/// for request processing.
static HANDLERS: &[Factory] = &[file::FileHandler::is, proxy::ProxyHandler::is];

/// Returns the first handler that can process the given request.
///
/// Iterates over all registered handler factories in `HANDLERS`.
/// Calls each factory with the provided `config` and `request`.
/// Returns `Some(Box<dyn Handler>)` if a suitable handler is found,
/// or `None` if no handler can handle the request.
///
/// # Examples
///
/// ```rust
/// let handler = get_handler(&config, &request);
/// if let Some(h) = handler {
///     let response = h.run(&request);
///     // process the response
/// }
/// ```
pub fn get_handler(config: &Config, request: &HttpRequest) -> Option<Box<dyn Handler>> {
    for h in HANDLERS {
        if let Some(handler) = h(config, request) {
            return Some(handler);
        }
    }
    None
}
