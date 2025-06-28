use std::{fs, path::PathBuf};

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
    let r = fs::read(path);
    if r.is_ok() { Some(r.unwrap()) } else { None }
}
//
// Suggest to use .ok()? instead of manual unwrap/if is_ok for more idiomatic error handling:
// fn serve_file(path: &PathBuf) -> Option<Vec<u8>> {
// fs::read(path).ok()
// }
//
//
