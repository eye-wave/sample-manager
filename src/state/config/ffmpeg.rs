use std::path::{Path, PathBuf};

pub fn find_executable(cmd: &str) -> Option<PathBuf> {
    which::which(cmd).ok()
}

pub fn is_executable(path: &Path) -> bool {
    let metadata = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return false,
    };

    if !metadata.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        metadata.permissions().mode() & 0o111 != 0
    }

    #[cfg(windows)]
    {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "exe")
            .unwrap_or(false)
    }
}
