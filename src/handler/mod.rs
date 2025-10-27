use crate::{
    handler::handler::{Handler, HandlerFactory},
    utils::Context,
};

pub mod file;
mod handler;
pub mod proxy;

/// Type alias for a handler factory function.
///
/// A factory takes a reference to the current `Config` and `HttpRequest`
/// and returns an `Option<Box<dyn Handler>>`. It returns `Some(handler)`
/// if it can handle the request, or `None` if it cannot.
type Factory = fn(&Context) -> Option<Box<dyn Handler>>;

/// List of all available handler factories.
///
/// New handlers can be added to this array to make them available
/// for request processing.
static HANDLERS: &[Factory] = &[proxy::ProxyHandler::is, file::FileHandler::is];

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

pub struct HandlerEngine {
    handlers: Vec<Factory>,
}

impl HandlerEngine {
    pub fn new() -> HandlerEngine {
        let mut handlers = Vec::new();
        handlers.extend_from_slice(HANDLERS);
        HandlerEngine { handlers }
    }

    pub fn add_handler(&mut self, handler: Factory) {
        self.handlers.insert(0, handler);
    }

    pub fn get_handler(&self, ctx: &Context) -> Option<Box<dyn Handler>> {
        for h in self.handlers.iter() {
            if let Some(handler) = h(ctx) {
                return Some(handler);
            }
        }
        None
    }
}
