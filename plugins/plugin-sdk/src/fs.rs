use crate::host;

/// Maximum file size the plugin will read in one call (4 MB).
/// If the file is larger the returned bytes are silently truncated —
/// for registry files this is more than enough.
const MAX_FILE_SIZE: usize = 4 * 1024 * 1024;

#[derive(Debug)]
pub enum FsError {
    /// Plugin does not have the `filesystem` capability.
    NotPermitted,
    /// The path string is not valid UTF-8 (shouldn't happen in practice).
    InvalidPath,
    /// Host rejected the path due to a `..` traversal sequence.
    PathTraversal,
    /// File not found or OS read error.
    ReadError,
    /// File content is not valid UTF-8.
    NotUtf8,
}

/// Read an entire file from the host filesystem into a `Vec<u8>`.
///
/// The plugin must declare `filesystem = true` in its manifest or this
/// will return `Err(FsError::NotPermitted)`.
pub fn read_file(path: &str) -> Result<Vec<u8>, FsError> {
    let mut buf = vec![0u8; MAX_FILE_SIZE];
    let n = unsafe {
        host::fs_read(
            path.as_ptr(),
            path.len() as u32,
            buf.as_mut_ptr(),
            buf.len() as u32,
        )
    };
    match n {
        n if n >= 0 => {
            buf.truncate(n as usize);
            Ok(buf)
        }
        -1 => Err(FsError::NotPermitted),
        -2 => Err(FsError::InvalidPath),
        _ => Err(FsError::ReadError),
    }
}

/// Read an entire file and decode it as UTF-8.
pub fn read_file_str(path: &str) -> Result<String, FsError> {
    let bytes = read_file(path)?;
    String::from_utf8(bytes).map_err(|_| FsError::NotUtf8)
}
