/// Represents an HTTP method (verb).
///
/// Includes standard HTTP/1.1 methods such as `GET`, `POST`, `PUT`, etc.,
/// and a catch-all variant `Other(String)` for unknown or non-standard methods.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
    TRACE,
    CONNECT,
    Other(String),
}
impl HttpMethod {
    /// Creates an `HttpMethod` from a raw string (case-sensitive).
    ///
    /// If the method is not one of the standard HTTP methods,
    /// it will be returned as `HttpMethod::Other(method.to_string())`.
    ///
    /// # Examples
    /// ```
    /// use hteapot::HttpMethod;
    ///
    /// let m = HttpMethod::from_str("GET");
    /// assert_eq!(m, HttpMethod::GET);
    ///
    /// let custom = HttpMethod::from_str("CUSTOM");
    /// assert_eq!(custom, HttpMethod::Other("CUSTOM".into()));
    /// ```
    pub fn from_str(method: &str) -> HttpMethod {
        let method = method.to_uppercase();
        match method.as_str() {
            "GET" => HttpMethod::GET,
            "POST" => HttpMethod::POST,
            "PUT" => HttpMethod::PUT,
            "DELETE" => HttpMethod::DELETE,
            "PATCH" => HttpMethod::PATCH,
            "HEAD" => HttpMethod::HEAD,
            "OPTIONS" => HttpMethod::OPTIONS,
            "TRACE" => HttpMethod::TRACE,
            "CONNECT" => HttpMethod::CONNECT,
            _ => Self::Other(method.to_string()),
        }
    }

    /// Returns the string representation of the HTTP method.
    ///
    /// If the method is non-standard (`Other`), it returns the inner string as-is.
    ///
    /// # Examples
    /// ```
    /// use hteapot::HttpMethod;
    ///
    /// let method = HttpMethod::GET;
    /// assert_eq!(method.to_str(), "GET");
    /// ```
    pub fn to_str(&self) -> &str {
        match self {
            HttpMethod::GET => "GET",
            HttpMethod::POST => "POST",
            HttpMethod::PUT => "PUT",
            HttpMethod::DELETE => "DELETE",
            HttpMethod::PATCH => "PATCH",
            HttpMethod::HEAD => "HEAD",
            HttpMethod::OPTIONS => "OPTIONS",
            HttpMethod::TRACE => "TRACE",
            HttpMethod::CONNECT => "CONNECT",
            HttpMethod::Other(method) => method.as_str(),
        }
    }
}

// #[derive(Clone, Copy)]
// pub enum Protocol {
//     HTTP,
//     HTTPS,
// }
