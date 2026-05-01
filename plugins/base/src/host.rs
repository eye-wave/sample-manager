#[link(wasm_import_module = "host")]
unsafe extern "C" {
    pub fn log(ptr: *const u8, len: u32);

    pub fn storage_get(key_ptr: *const u8, key_len: u32, out_ptr: *mut u8, out_cap: u32) -> u32;

    pub fn storage_set(key_ptr: *const u8, key_len: u32, val_ptr: *const u8, val_len: u32);

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
