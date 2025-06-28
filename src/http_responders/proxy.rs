use crate::config::Config;
use crate::hteapot::HttpRequest;

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
pub fn is_proxy(config: &Config, req: HttpRequest) -> Option<(String, HttpRequest)> {
    for proxy_path in config.proxy_rules.keys() {
        let path_match = req.path.strip_prefix(proxy_path);
        if path_match.is_some() {
            let new_path = path_match.unwrap();
            let url = config.proxy_rules.get(proxy_path).unwrap().clone();
            let mut proxy_req = req.clone();
            proxy_req.path = new_path.to_string();
            proxy_req.headers.remove("host");
            proxy_req.headers.remove("Host");
            let host_parts: Vec<_> = url.split("://").collect();
            let host = if host_parts.len() == 1 {
                host_parts.first().unwrap()
            } else {
                host_parts.last().clone().unwrap()
            };
            proxy_req.header("host", host);
            return Some((url, proxy_req));
        }
    }
    None
}
