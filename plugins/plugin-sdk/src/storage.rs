use crate::host;

const DEFAULT_BUF: usize = 4096;

pub fn storage_get(key: &str) -> Option<Vec<u8>> {
    let mut buf = vec![0u8; DEFAULT_BUF];
    let n = unsafe {
        host::storage_get(
            key.as_ptr(),
            key.len() as u32,
            buf.as_mut_ptr(),
            buf.len() as u32,
        )
    };
    if n == u32::MAX {
        return None;
    }
    buf.truncate(n as usize);
    Some(buf)
}

pub fn storage_set(key: &str, value: &[u8]) {
    unsafe {
        host::storage_set(
            key.as_ptr(),
            key.len() as u32,
            value.as_ptr(),
            value.len() as u32,
        )
    }
}

pub fn storage_get_str(key: &str) -> Option<String> {
    storage_get(key).and_then(|b| String::from_utf8(b).ok())
}

pub fn storage_set_str(key: &str, value: &str) {
    storage_set(key, value.as_bytes());
}

pub fn secret_get(key: &str) -> Option<Vec<u8>> {
    let mut buf = vec![0u8; DEFAULT_BUF];
    let n = unsafe {
        host::secret_get(
            key.as_ptr(),
            key.len() as u32,
            buf.as_mut_ptr(),
            buf.len() as u32,
        )
    };
    if n == u32::MAX {
        return None;
    }
    buf.truncate(n as usize);
    Some(buf)
}

pub fn secret_set(key: &str, value: &[u8]) {
    unsafe {
        host::secret_set(
            key.as_ptr(),
            key.len() as u32,
            value.as_ptr(),
            value.len() as u32,
        )
    }
}

pub fn secret_get_str(key: &str) -> Option<String> {
    secret_get(key).and_then(|b| String::from_utf8(b).ok())
}

pub fn secret_set_str(key: &str, value: &str) {
    secret_set(key, value.as_bytes());
}
