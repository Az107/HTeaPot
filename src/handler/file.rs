use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    handler::handler::{Handler, HandlerFactory},
    headers,
    hteapot::{HttpResponse, HttpStatus},
    utils::{Context, get_mime_tipe},
};

/// Attempts to safely join a root directory and a requested relative path.
///
/// Ensures that the resulting path:
/// - Resolves symbolic links and `..` segments via `canonicalize`
/// - Remains within the bounds of the specified root directory
/// - Actually exists on disk
///
/// This protects against directory traversal vulnerabilities, such as accessing
/// files outside of the intended root (e.g., `/etc/passwd`).
///
/// # Arguments
/// * `root` - The root directory from which serving is allowed.
/// * `requested_path` - The path requested by the client (usually from the URL).
///
/// # Returns
/// `Some(PathBuf)` if the resolved path exists and is within the root. `None` otherwise.
///
/// # Example
/// ```
/// let safe_path = safe_join_paths("/var/www", "/index.html");
/// assert!(safe_path.unwrap().ends_with("index.html"));
/// ```
pub fn safe_join_paths(root: &str, requested_path: &str) -> Option<PathBuf> {
    let root_path = Path::new(root).canonicalize().ok()?;
    let requested_full_path = root_path.join(requested_path.trim_start_matches("/"));

    if !requested_full_path.exists() {
        return None;
    }

    let canonical_path = requested_full_path.canonicalize().ok()?;

    if canonical_path.starts_with(&root_path) {
        Some(canonical_path)
    } else {
        None
    }
}

pub struct FileHandler {
    root: String,
    index: String,
}

impl FileHandler {}

impl Handler for FileHandler {
    fn run(&self, ctx: &mut Context) -> Box<dyn crate::hteapot::HttpResponseCommon> {
        let logger = ctx.log.with_component("HTTP");
        // If the request is not a proxy request, resolve the requested path safely
        let safe_path_result = if ctx.request.path == "/" {
            // Special handling for the root "/" path
            let root_path = Path::new(&self.root).canonicalize();
            if root_path.is_ok() {
                // If the root path exists and is valid, try to join the index file
                let index_path = root_path.unwrap().join(&self.index);
                if index_path.exists() {
                    Some(index_path) // If index exists, return its path
                } else {
                    None // If no index exists, return None
                }
            } else {
                None // If the root path is invalid, return None
            }
        } else {
            // For any other path, resolve it safely using the `safe_join_paths` function
            safe_join_paths(&self.root, &ctx.request.path)
        };

        // Handle the case where the resolved path is a directory
        let safe_path = match safe_path_result {
            Some(path) => {
                if path.is_dir() {
                    // If it's a directory, check for the index file in that directory
                    let index_path = path.join(&self.index);
                    if index_path.exists() {
                        index_path // If index exists, return its path
                    } else {
                        // If no index file exists, log a warning and return a 404 response
                        logger.warn(format!(
                            "Index file not found in directory: {}",
                            ctx.request.path
                        ));
                        return HttpResponse::new(HttpStatus::NotFound, "Index not found", None);
                    }
                } else {
                    path // If it's not a directory, just return the path
                }
            }
            None => {
                // If the path is invalid or access is denied, return a 404 response
                logger.warn(format!(
                    "Path not found or access denied: {}",
                    ctx.request.path
                ));
                return HttpResponse::new(HttpStatus::NotFound, "Not found", None);
            }
        };

        // Determine the MIME type for the file based on its extension
        let mimetype = get_mime_tipe(&safe_path.to_string_lossy().to_string());

        // Try to serve the file from the cache, or read it from disk if not cached
        let content = fs::read(&safe_path).ok();
        match content {
            Some(c) => {
                // If content is found, create response with proper headers and a 200 OK status
                let headers = headers!(
                    "Content-Type" => &mimetype,
                    "X-Content-Type-Options" => "nosniff"
                );
                let response = HttpResponse::new(HttpStatus::OK, c, headers);
                if ctx.cache.is_some() {
                    let cache = ctx.cache.as_deref_mut().unwrap();
                    cache.set(ctx.request.clone(), (*response).clone());
                }
                response
            }
            None => {
                // If no content is found, return a 404 Not Found response
                HttpResponse::new(HttpStatus::NotFound, "Not found", None)
            }
        }
    }
}

impl HandlerFactory for FileHandler {
    fn is(ctx: &Context) -> Option<Box<dyn Handler>> {
        Some(Box::new(FileHandler {
            root: ctx.config.root.to_string(),
            index: ctx.config.index.to_string(),
        }))
    }
}
