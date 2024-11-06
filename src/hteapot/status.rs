#[derive(Clone, Copy)]
pub enum HttpStatus {
    OK = 200,
    Created = 201,
    Accepted = 202,
    NoContent = 204,
    MovedPermanently = 301,
    MovedTemporarily = 302,
    NotModified = 304,
    BadRequest = 400,
    Unauthorized = 401,
    Forbidden = 403,
    NotFound = 404,
    IAmATeapot = 418,
    InternalServerError = 500,
    NotImplemented = 501,
    BadGateway = 502,
    ServiceUnavailable = 503,
}

impl HttpStatus {
    pub fn from_u16(status: u16) -> HttpStatus {
        match status {
            200 => HttpStatus::OK,
            201 => HttpStatus::Created,
            202 => HttpStatus::Accepted,
            204 => HttpStatus::NoContent,
            301 => HttpStatus::MovedPermanently,
            302 => HttpStatus::MovedTemporarily,
            304 => HttpStatus::NotModified,
            400 => HttpStatus::BadRequest,
            401 => HttpStatus::Unauthorized,
            403 => HttpStatus::Forbidden,
            404 => HttpStatus::NotFound,
            418 => HttpStatus::IAmATeapot,
            500 => HttpStatus::InternalServerError,
            501 => HttpStatus::NotImplemented,
            502 => HttpStatus::BadGateway,
            503 => HttpStatus::ServiceUnavailable,
            _ => panic!("Invalid HTTP status"),
        }
    }

    pub fn to_string(&self) -> &str {
        match self {
            HttpStatus::OK => "OK",
            HttpStatus::Created => "Created",
            HttpStatus::Accepted => "Accepted",
            HttpStatus::NoContent => "No Content",
            HttpStatus::MovedPermanently => "Moved Permanently",
            HttpStatus::MovedTemporarily => "Moved Temporarily",
            HttpStatus::NotModified => "Not Modified",
            HttpStatus::BadRequest => "Bad Request",
            HttpStatus::Unauthorized => "Unauthorized",
            HttpStatus::Forbidden => "Forbidden",
            HttpStatus::NotFound => "Not Found",
            HttpStatus::IAmATeapot => "I'm a teapot",
            HttpStatus::InternalServerError => "Internal Server Error",
            HttpStatus::NotImplemented => "Not Implemented",
            HttpStatus::BadGateway => "Bad Gateway",
            HttpStatus::ServiceUnavailable => "Service Unavailable",
        }
    }
}
