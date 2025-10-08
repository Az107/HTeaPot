use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    handler::handler::{Handler, HandlerFactory},
    hteapot::{HttpHeaders, HttpResponse, HttpStatus},
    utils::{Context, get_mime_tipe},
};

/// Safely joins a root directory with a requested relative path.
///
/// Ensures that:
/// - Symbolic links and `..` segments are resolved (`canonicalize`)
/// - The resulting path stays within `root`
/// - The path exists on disk
///
/// This prevents directory traversal attacks (e.g., accessing `/etc/passwd`).
///
/// # Arguments
/// * `root` - Allowed root directory.
/// * `requested_path` - Path requested by the client.
///
/// # Returns
/// `Some(PathBuf)` if the path is valid and exists, `None` otherwise.
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

/// Handles serving static files from a root directory, including index files.
pub struct FileHandler {
    root: String,
    index: String,
}

impl FileHandler {}

impl Handler for FileHandler {
    fn run(&self, ctx: &mut Context) -> Box<dyn crate::hteapot::HttpResponseCommon> {
        let logger = ctx.log.with_component("HTTP");

        // Resolve the requested path safely
        let safe_path_result = if ctx.request.path == "/" {
            // Special handling for the root path: serve the index file
            Path::new(&self.root)
                .canonicalize()
                .ok()
                .map(|root_path| root_path.join(&self.index))
                .filter(|index_path| index_path.exists())
        } else {
            // Other paths: use safe join
            safe_join_paths(&self.root, &ctx.request.path)
        };

        // Handle directories or invalid paths
        let safe_path = match safe_path_result {
            Some(path) => {
                if path.is_dir() {
                    let index_path = path.join(&self.index);
                    if index_path.exists() {
                        index_path
                    } else {
                        logger.warn(format!(
                            "Index file not found in directory: {}",
                            ctx.request.path
                        ));
                        return HttpResponse::new(HttpStatus::NotFound, "Index not found", None);
                    }
                } else {
                    path
                }
            }
            None => {
                logger.warn(format!(
                    "Path not found or access denied: {}",
                    ctx.request.path
                ));
                return HttpResponse::new(HttpStatus::NotFound, "Not found", None);
            }
        };

        // Determine MIME type
        let mimetype = get_mime_tipe(&safe_path.to_string_lossy().to_string());

        // Read file content
        match fs::read(&safe_path).ok() {
            Some(content) => {
                let mut headers = HttpHeaders::new();
                headers.insert("Content-Type", &mimetype);
                headers.insert("X-Content-Type-Options", "nosniff");
                let response = HttpResponse::new(HttpStatus::OK, content, Some(headers));

                // Cache the response if caching is enabled
                if let Some(cache) = ctx.cache.as_deref_mut() {
                    cache.set(ctx.request.clone(), (*response).clone());
                }

                response
            }
            None => HttpResponse::new(HttpStatus::NotFound, "Not found", None),
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
