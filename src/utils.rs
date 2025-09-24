use std::path::Path;

use crate::{
    cache::Cache,
    config::Config,
    hteapot::{HttpRequest, HttpResponse},
    logger::Logger,
};

/// Returns the MIME type based on the file extension of a given path.
///
/// This function maps common file extensions to their appropriate
/// `Content-Type` values for HTTP responses.
///
/// If the extension is unrecognized or missing, it defaults to
/// `"application/octet-stream"` for safe binary delivery.
///
/// # Arguments
///
/// * `path` - A file path as a `String` from which to extract the extension.
///
/// # Examples
///
/// ```
/// let mime = get_mime_tipe(&"file.html".to_string());
/// assert_eq!(mime, "text/html; charset=utf-8");
/// ```
pub fn get_mime_tipe(path: &String) -> String {
    let extension = Path::new(path.as_str())
        .extension()
        .map(|ext| ext.to_str().unwrap_or(""))
        .unwrap_or("");

    // Suggest using `to_str()` directly on the extension
    // Alternative way to get the extension
    // .and_then(|ext| ext.to_str())

    let mimetipe = match extension {
        // Text
        "html" | "htm" | "php" => "text/html; charset=utf-8",
        "js" => "text/javascript",
        "mjs" => "text/javascript",
        "css" => "text/css",
        "json" => "application/json",
        "xml" => "application/xml",
        "txt" => "text/plain",
        "md" => "text/markdown",
        "csv" => "text/csv",

        // Images
        "ico" => "image/x-icon",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "tiff" | "tif" => "image/tiff",

        // Audio
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "flac" => "audio/flac",

        // Video
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "avi" => "video/x-msvideo",
        "mkv" => "video/x-matroska",

        // Documents
        "pdf" => "application/pdf",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",

        // Archives
        "zip" => "application/zip",
        "tar" => "application/x-tar",
        "gz" => "application/gzip",
        "7z" => "application/x-7z-compressed",
        "rar" => "application/vnd.rar",

        // Fonts
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "woff" => "font/woff",
        "woff2" => "font/woff2",

        // For unknown types, use a safe default
        _ => "application/octet-stream",
    };

    mimetipe.to_string()
}

//TODO: make a parser args to config
//pub fn args_to_dict(list: Vec<String>) -> HashMap<String, String> {}

pub struct Context<'a> {
    pub request: &'a HttpRequest,
    pub log: &'a Logger,
    pub config: &'a Config,
    pub cache: Option<&'a mut Cache<HttpRequest, HttpResponse>>,
}
