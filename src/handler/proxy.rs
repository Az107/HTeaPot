use crate::handler::handler::{Handler, HandlerFactory};
use crate::hteapot::{HttpMethod, HttpResponse, HttpResponseCommon, HttpStatus, TunnelResponse};
use crate::utils::Context;

/// Determines whether a given HTTP request should be proxied based on the configuration.
///
/// If a matching proxy rule is found in `config.proxy_rules`, the function rewrites the
/// request path and updates the `Host` header accordingly.
///
/// # Arguments
/// * `config` - Server configuration containing proxy rules.
/// * `req` - The original HTTP request.
///
/// # Returns
/// `Some((proxy_url, modified_request))` if the request should be proxied, otherwise `None`.

pub struct ProxyHandler {
    new_path: String,
    url: String,
}

impl Handler for ProxyHandler {
    fn run(&self, ctx: &mut Context) -> Box<dyn HttpResponseCommon> {
        let proxy_logger = &ctx.log.with_component("proxy");
        if ctx.request.method == HttpMethod::OPTIONS {
            return TunnelResponse::new(&ctx.request.path);
        }
        let mut proxy_req = ctx.request.clone();
        proxy_req.path = self.new_path.clone();
        proxy_req.headers.remove("Host");
        let host_parts: Vec<&str> = self.url.split("://").collect();
        let host = if host_parts.len() == 1 {
            host_parts.first().unwrap()
        } else {
            host_parts.last().clone().unwrap()
        };
        proxy_req.headers.insert("host", host);
        let response = proxy_req.brew(&self.url).unwrap_or(HttpResponse::new(
            HttpStatus::NotAcceptable,
            "",
            None,
        ));
        if ctx.cache.is_some() {
            let cache = ctx.cache.as_deref_mut().unwrap();
            cache.set(ctx.request.clone(), (*response).clone());
        }
        response
    }
}

impl HandlerFactory for ProxyHandler {
    fn is(ctx: &Context) -> Option<Box<dyn Handler>> {
        if ctx.request.method == HttpMethod::OPTIONS {
            return Some(Box::new(ProxyHandler {
                url: String::new(),
                new_path: String::new(),
            }));
        }
        for proxy_path in ctx.config.proxy_rules.keys() {
            let path_match = ctx.request.path.strip_prefix(proxy_path);
            if path_match.is_some() {
                let new_path = path_match.unwrap().to_string();
                let url = &ctx.config.proxy_rules.get(proxy_path).unwrap().clone();
                let url = if url.is_empty() {
                    let proxy_url = &ctx.request.headers.get("host")?;
                    proxy_url.to_owned()
                } else {
                    url
                }
                .to_string();
                return Some(Box::new(ProxyHandler { url, new_path }));
            }
        }
        return None;
    }
}
