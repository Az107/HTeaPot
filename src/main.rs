mod cache;
mod config;
pub mod hteapot;
mod logger;
mod utils;

use std::{fs, io};

use std::path::Path;
use std::process::Command;

use std::sync::Mutex;

use cache::Cache;
use config::Config;
use hteapot::{Hteapot, HttpRequest, HttpResponse, HttpStatus};
use utils::get_mime_tipe;

use logger::Logger;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn is_proxy(config: &Config, req: HttpRequest) -> Option<(String, HttpRequest)> {
    for proxy_path in config.proxy_rules.keys() {
        let path_match = req.path.strip_prefix(proxy_path);
        if path_match.is_some() {
            let new_path = path_match.unwrap();
            let url = config.proxy_rules.get(proxy_path).unwrap().clone();
            let mut proxy_req = req.clone();
            proxy_req.path = new_path.to_string();
            proxy_req.headers.remove("Host");
            let host_parts: Vec<_> = url.split("://").collect();
            let host = if host_parts.len() == 1 {
                host_parts.first().unwrap()
            } else {
                host_parts.last().clone().unwrap()
            };
            proxy_req.header("Host", host);
            return Some((url, proxy_req));
        }
    }
    None
}

fn serve_file(path: &String) -> Option<Vec<u8>> {
    let r = fs::read(path);
    if r.is_ok() { Some(r.unwrap()) } else { None }
}

#[cfg(feature = "cgi")]

fn serve_cgi(
    program: &String,
    path: &String,
    request: HttpRequest,
) -> Result<Vec<u8>, &'static str> {
    use std::{env, io::Write, process::Stdio};
    let query = request
        .args
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&");
    unsafe {
        //TODO: !! fix this, avoid using unsafe , this could conflict simultaneous CGI executions, change to fastCGI ?
        env::set_var("REDIRECT_STATUS", "hteapot");
        env::set_var("SCRIPT_NAME", path);
        env::set_var("SCRIPT_FILENAME", path);
        env::set_var("QUERY_STRING", query);
        env::set_var("REQUEST_METHOD", request.method.to_str());
    }
    let content_type = request.headers.get("CONTENT_TYPE");
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
        .arg(&path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn child process");

    let stdin = child.stdin.as_mut().expect("msg");
    stdin
        .write_all(request.body.as_slice())
        .expect("Error writing stdin");
    let output = child.wait_with_output();
    match output {
        Ok(output) => {
            if output.status.success() {
                Ok(output.stdout)
            } else {
                Err("Command exit with non-zero status")
            }
        }
        Err(_) => Err("Error runing command"),
    }
}

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() == 1 {
        println!("Hteapot {}", VERSION);
        println!("usage: {} <config file>", args[0]);
        return;
    }
    let config = match args[1].as_str() {
        "--help" | "-h" => {
            println!("Hteapot {}", VERSION);
            println!("usage: {} <config file>", args[0]);
            return;
        }
        "--version" | "-v" => {
            println!("Hteapot {}", VERSION);
            return;
        }
        "--serve" | "-s" => {
            let mut c = config::Config::new_default();
            let serving_path = Some(args.get(2).unwrap().clone());
            let serving_path_str = serving_path.unwrap();
            let serving_path_str = serving_path_str.as_str();
            let serving_path = Path::new(serving_path_str);
            if serving_path.is_dir() {
                c.root = serving_path.to_str().unwrap_or_default().to_string();
            } else {
                c.index = serving_path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap_or_default()
                    .to_string();
                c.root = serving_path
                    .parent()
                    .unwrap_or(Path::new("./"))
                    .to_str()
                    .unwrap_or_default()
                    .to_string();
            }
            c.host = "0.0.0.0".to_string();
            c
        }
        _ => config::Config::load_config(&args[1]),
    };

    let proxy_only = config.proxy_rules.get("/").is_some();
    let logger = match config.log_file.clone() {
        Some(file_name) => {
            let file = fs::File::create(file_name.clone());
            let file = file.unwrap();
            Logger::new(file)
        }
        None => Logger::new(io::stdout()),
    };

    let cache: Mutex<Cache> = Mutex::new(Cache::new(config.cache_ttl as u64));
    let server = Hteapot::new_threaded(config.host.as_str(), config.port, config.threads);
    logger.msg(format!(
        "Server started at http://{}:{}",
        config.host, config.port
    ));
    if config.cache {
        logger.msg("Cache Enabled".to_string());
    }
    if proxy_only {
        logger
            .msg("WARNING: All requests are proxied to /. Local paths won’t be used.".to_string());
    }
    server.listen(move |req| {
        // SERVER CORE
        // for each request
        logger.msg(format!("Request {} {}", req.method.to_str(), req.path));
        let is_proxy = is_proxy(&config, req.clone());

        if proxy_only || is_proxy.is_some() {
            let (host, proxy_req) = is_proxy.unwrap();
            let res = proxy_req.brew(host.as_str());
            if res.is_ok() {
                return res.unwrap();
            } else {
                return HttpResponse::new(
                    HttpStatus::InternalServerError,
                    "Internal Server Error",
                    None,
                );
            }
        }

        let mut full_path = format!("{}{}", config.root, req.path.clone());
        if Path::new(full_path.as_str()).is_dir() {
            let separator = if full_path.ends_with('/') { "" } else { "/" };
            full_path = format!("{}{}{}", full_path, separator, config.index);
        }

        if !Path::new(full_path.as_str()).exists() {
            logger.msg(format!("path {} does not exist", req.path));
            return HttpResponse::new(HttpStatus::NotFound, "Not found", None);
        }

        #[cfg(feature = "cgi")]
        {
            let extension = Path::new(&full_path).extension().unwrap();
            let extension = extension.to_str().unwrap();
            println!("File extension: {}", extension);
            let cgi_command = config.cgi_rules.get(extension);
            if cgi_command.is_some() {
                let cgi_command = cgi_command.unwrap();
                logger.msg(format!("Runing {} {}", cgi_command, full_path));
                let cgi_result = serve_cgi(cgi_command, &full_path, req);

                return match cgi_result {
                    Ok(result) => HttpResponse::new(HttpStatus::OK, result, None),
                    Err(err) => HttpResponse::new(
                        HttpStatus::InternalServerError,
                        "Internal server error",
                        None,
                    ),
                };
            }
        }

        let mimetype = get_mime_tipe(&full_path);
        let content: Option<Vec<u8>> = if config.cache {
            let mut cachee = cache.lock().expect("Error locking cache");
            let mut r = cachee.get(req.path.clone());
            if r.is_none() {
                r = serve_file(&full_path);
                if r.is_some() {
                    cachee.set(req.path.clone(), r.clone().unwrap());
                }
            }
            r
        } else {
            serve_file(&full_path)
        };
        match content {
            Some(c) => HttpResponse::new(HttpStatus::OK, c, headers!("Content-Type" => mimetype)),
            None => HttpResponse::new(HttpStatus::NotFound, "Not found", None),
        }
    });
}
