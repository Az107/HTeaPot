use std::{
    fs,
    path::{Path, PathBuf},
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

/// Reads the content of a file from the filesystem.
///
/// # Arguments
/// * `path` - A reference to a `PathBuf` representing the target file.
///
/// # Returns
/// `Some(Vec<u8>)` if the file is read successfully, or `None` if an error occurs.
///
/// # Notes
/// Uses `PathBuf` instead of `&str` to clearly express intent and reduce path handling bugs.
///
/// # See Also
/// [`std::fs::read`](https://doc.rust-lang.org/std/fs/fn.read.html)
pub fn serve_file(path: &PathBuf) -> Option<Vec<u8>> {
    fs::read(path).ok()
}
