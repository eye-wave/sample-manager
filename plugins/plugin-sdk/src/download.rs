use crate::host;

pub fn emit_download(bytes: &[u8], filename: &str) -> Result<(), i32> {
    let n = unsafe {
        host::emit_download(
            bytes.as_ptr(),
            bytes.len() as u32,
            filename.as_ptr(),
            filename.len() as u32,
        )
    };
    if n == 0 { Ok(()) } else { Err(n) }
}
