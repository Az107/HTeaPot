use super::HttpMethod;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub args: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl HttpRequest {
    pub fn default() -> Self {
        HttpRequest {
            method: HttpMethod::GET,
            path: String::new(),
            args: HashMap::new(),
            headers: HashMap::new(),
            body: String::new(),
        }
    }
}

pub struct HttpRequestBuilder {
    request: HttpRequest,
    buffer: Vec<u8>,
    done: bool,
}

impl HttpRequestBuilder {
    pub fn new() -> Self {
        return HttpRequestBuilder {
            request: HttpRequest {
                method: HttpMethod::GET,
                path: String::new(),
                args: HashMap::new(),
                headers: HashMap::new(),
                body: String::new(),
            },
            buffer: Vec::new(),
            done: false,
        };
    }

    pub fn append(&mut self, buffer: Vec<u8>) -> Option<HttpRequest> {
        self.buffer.extend(buffer);

        while let Some(pos) = self.buffer.windows(2).position(|w| w == b"\r\n") {
            let line = self.buffer.drain(..pos).collect::<Vec<u8>>(); // Extraer línea
            self.buffer.drain(..2); // Eliminar `\r\n`

            let line_str = String::from_utf8_lossy(&line);

            if self.request.path.is_empty() {
                // Primera línea: Método + Path + Versión HTTP
                let parts: Vec<&str> = line_str.split_whitespace().collect();
                if parts.len() < 2 {
                    return None; // Request malformada
                }

                self.request.method = HttpMethod::from_str(parts[0]); // Convierte a enum
                let path_parts: Vec<&str> = parts[1].split('?').collect();
                self.request.path = path_parts[0].to_string();

                // Si hay argumentos en la URL, los parseamos
                if path_parts.len() > 1 {
                    self.request.args = path_parts[1]
                        .split('&')
                        .filter_map(|pair| {
                            let kv: Vec<&str> = pair.split('=').collect();
                            if kv.len() == 2 {
                                Some((kv[0].to_string(), kv[1].to_string()))
                            } else {
                                None
                            }
                        })
                        .collect();
                }
            } else if !line_str.is_empty() {
                // Cabeceras HTTP
                if let Some((key, value)) = line_str.split_once(": ") {
                    self.request
                        .headers
                        .insert(key.to_string(), value.to_string());
                }
            } else {
                // Fin de las cabeceras
                self.done = true;
                return Some(std::mem::replace(&mut self.request, HttpRequest::default()));
            }
        }

        None // Aún no tenemos toda la request
    }
}
