use std::collections::HashMap;
use std::{
    env,
    io::Write,
    process::{Command, Stdio},
};

use crate::hteapot::{HttpRequest, HttpStatus};

#[cfg(feature = "cgi")]
pub fn serve_cgi(
    program: String,
    file_dir: String,
    file_name: String,
    request: HttpRequest,
) -> Result<(HttpStatus, HashMap<String, String>, Vec<u8>), &'static str> {
    use std::{
        env,
        io::Write,
        process::{Command, Stdio},
    };
    //THIS LINES ONLY EXIST COS WINDOWS >:C
    let file_dir = if file_dir.starts_with("\\\\?\\") {
        file_dir.strip_prefix("\\\\?\\").unwrap().to_string()
    } else {
        file_dir.to_owned()
    };
    let file_dir = file_dir.replace("\\", "/");
    let file_name = file_name.replace("\\", "/");

    let query = request
        .args
        .iter()
        .map(|(key, value)| {
            if value.is_empty() {
                key.to_owned()
            } else {
                format!("{key}={value}")
            }
        })
        .collect::<Vec<_>>()
        .join("&");
    unsafe {
        //TODO: !! fix this, avoid using unsafe , this could conflict simultaneous CGI executions, change to fastCGI ?
        env::set_var("REDIRECT_STATUS", "200");
        // 1. Información del script y request
        env::set_var("GATEWAY_INTERFACE", "CGI/1.1");
        env::set_var("SERVER_PROTOCOL", "HTTP/1.1"); // ej. "HTTP/1.1"
        env::set_var("REQUEST_METHOD", request.method.to_str());
        env::set_var("QUERY_STRING", &query);
        env::set_var("REQUEST_URI", format!("{}?{}", request.path, &query));

        if let Some(cookies) = request.headers.get("cookie") {
            env::set_var("HTTP_COOKIE", cookies);
        }

        // SCRIPT_NAME = ruta relativa al docroot
        // SCRIPT_FILENAME = ruta absoluta al script en disco
        env::set_var("SCRIPT_NAME", &file_name);
        env::set_var("SCRIPT_FILENAME", format!("{}{}", &file_dir, &file_name));

        // PATH_INFO es opcional, solo si usas /index.php/loquesea
        env::set_var(
            "PATH_INFO",
            request.path.strip_prefix(&file_name).unwrap_or("/"),
        );

        // Server info
        env::set_var("SERVER_NAME", "localhost"); // ej. "localhost" change to get from config
        env::set_var("SERVER_PORT", "8081"); // ej. "8080"
        env::set_var("HTTP_HOST", "localhost:8081"); // ej. "8080"
        env::set_var("SERVER_SOFTWARE", "hteapot/0.6.5");
        env::set_var("REMOTE_ADDR", "localhost"); // this should obtain the real address ?
    }

    let content_type = request.headers.get("content-type");
    let content_type = match content_type {
        Some(s) => s.clone(),
        None => "".to_string(),
    };

    unsafe {
        //TODO: !! fix this, avoid using unsafe , this could conflict simultaneous CGI executions, change to fastCGI ?
        env::set_var("CONTENT_TYPE", content_type); // Tipo de contenido
        env::set_var("CONTENT_LENGTH", request.body.len().to_string().as_str()); // Longitud del contenido para POST
    }
    let mut child = Command::new(program)
        .current_dir(&file_dir)
        .arg(&file_name)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn child process");

    let stdin = child.stdin.as_mut().expect("msg");
    stdin.write_all(&request.body).expect("Error writing stdin");
    let output = child.wait_with_output();
    match output {
        Ok(output) => {
            if let Some(pos) = output.stdout.windows(4).position(|w| w == b"\r\n\r\n") {
                use std::collections::HashMap;
                let mut status = HttpStatus::OK;
                let (raw_headers, second_with_sep) = output.stdout.split_at(pos);
                let body = &second_with_sep[4..];
                let raw_headers = String::from_utf8(raw_headers.to_vec()).unwrap();
                let mut headers = HashMap::new();
                for item in raw_headers.split("\n") {
                    let (k, v) = item.split_once(":").unwrap();
                    let k = k.trim().to_string();
                    let v = v.trim().to_string();
                    if k.to_lowercase() == "status" {
                        let status_code: u16 = v.split_once(' ').unwrap().0.parse().unwrap();
                        status = HttpStatus::from_u16(status_code).unwrap();
                        continue;
                    }
                    headers.insert(k.trim().to_string(), v.trim().to_string());
                }
                return Ok((status, headers, body.to_vec()));
            }
            Ok((HttpStatus::OK, HashMap::new(), output.stdout))
        }
        Err(_) => Err("Error runing command"),
    }
}
