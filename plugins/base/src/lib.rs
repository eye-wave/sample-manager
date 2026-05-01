unsafe extern "C" {
    pub fn log(ptr: *const u8, len: u32);

    pub fn secret_get(key_ptr: *const u8, key_len: u32, out_ptr: *mut u8, out_cap: u32) -> u32;

    pub fn secret_set(key_ptr: *const u8, key_len: u32, val_ptr: *const u8, val_len: u32);

    pub fn http_fetch(
        url_ptr: *const u8,
        url_len: u32,
        headers_ptr: *const u8,
        n_headers: u32,
        out_ptr: *mut u8,
        out_cap: u32,
    ) -> i32;
}

pub fn host_log(msg: &str) {
    unsafe { log(msg.as_ptr(), msg.len() as u32) }
}

pub fn host_secret_get(key: &str) -> Option<Vec<u8>> {
    let mut buf = vec![0u8; 4096];
    let n = unsafe {
        secret_get(
            key.as_ptr(),
            key.len() as u32,
            buf.as_mut_ptr(),
            buf.len() as u32,
        )
    };
    if n == u32::MAX {
        None
    } else {
        buf.truncate(n as usize);
        Some(buf)
    }
}

pub fn host_secret_set(key: &str, value: &[u8]) {
    unsafe {
        secret_set(
            key.as_ptr(),
            key.len() as u32,
            value.as_ptr(),
            value.len() as u32,
        )
    }
}

pub fn host_http_fetch(url: &str, headers: &[(&str, &str)]) -> Option<Vec<u8>> {
    let mut header_buf = Vec::with_capacity(headers.len() * 16);
    for (k, v) in headers {
        let k_ptr = k.as_ptr() as u32;
        let v_ptr = v.as_ptr() as u32;
        header_buf.extend_from_slice(&k_ptr.to_le_bytes());
        header_buf.extend_from_slice(&(k.len() as u32).to_le_bytes());
        header_buf.extend_from_slice(&v_ptr.to_le_bytes());
        header_buf.extend_from_slice(&(v.len() as u32).to_le_bytes());
    }

    let mut out = vec![0u8; 512 * 1024];
    let n = unsafe {
        http_fetch(
            url.as_ptr(),
            url.len() as u32,
            header_buf.as_ptr(),
            headers.len() as u32,
            out.as_mut_ptr(),
            out.len() as u32,
        )
    };
    if n < 0 {
        return None;
    }
    out.truncate(n as usize);
    Some(out)
}
