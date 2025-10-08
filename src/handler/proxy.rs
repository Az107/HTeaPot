use crate::handler::handler::{Handler, HandlerFactory};
use crate::hteapot::{HttpMethod, HttpResponse, HttpResponseCommon, HttpStatus, TunnelResponse};
use crate::utils::Context;

/// Handles HTTP proxying based on server configuration.
///
/// Determines whether a request matches any proxy rules and forwards it
/// to the corresponding upstream server, rewriting the path and `Host` header
/// as needed.
///
/// # Fields
/// * `new_path` - Path to use for the proxied request.
/// * `url` - Target upstream URL.
pub struct ProxyHandler {
    new_path: String,
    url: String,
}

impl Handler for ProxyHandler {
    fn run(&self, ctx: &mut Context) -> Box<dyn HttpResponseCommon> {
        let _proxy_logger = &ctx.log.with_component("proxy");

        // Return a tunnel response immediately for OPTIONS requests
        if ctx.request.method == HttpMethod::OPTIONS {
            return TunnelResponse::new(&ctx.request.path);
        }

        // Prepare a modified request for proxying
        let mut proxy_req = ctx.request.clone();
        proxy_req.path = self.new_path.clone();
        proxy_req.headers.remove("Host");

        // Determine the upstream host from the URL
        let host_parts: Vec<&str> = self.url.split("://").collect();
        let host = if host_parts.len() == 1 {
            host_parts.first().unwrap()
        } else {
            host_parts.last().unwrap()
        };
        proxy_req.headers.insert("host", host);

        // Forward the request and handle errors
        let response = proxy_req.brew(&self.url).unwrap_or(HttpResponse::new(
            HttpStatus::NotAcceptable,
            "",
            None,
        ));

        // Cache the response if caching is enabled
        if let Some(cache) = ctx.cache.as_deref_mut() {
            cache.set(ctx.request.clone(), (*response).clone());
        }

        response
    }
}

impl HandlerFactory for ProxyHandler {
    fn is(ctx: &Context) -> Option<Box<dyn Handler>> {
        // OPTIONS requests are always handled
        if ctx.request.method == HttpMethod::OPTIONS {
            return Some(Box::new(ProxyHandler {
                url: String::new(),
                new_path: String::new(),
            }));
        }

        // Check if the request matches any configured proxy rules
        for proxy_path in ctx.config.proxy_rules.keys() {
            if let Some(path_match) = ctx.request.path.strip_prefix(proxy_path) {
                let new_path = path_match.to_string();
                let url = ctx.config.proxy_rules.get(proxy_path).unwrap();
                let url = if url.is_empty() {
                    // If the rule URL is empty, fallback to Host header
                    let proxy_url = ctx.request.headers.get("host")?;
                    proxy_url.to_owned()
                } else {
                    url.to_string()
                };

                return Some(Box::new(ProxyHandler { url, new_path }));
            }
        }

        None
    }
}
