/// Write [u32 len LE][bytes] into a new allocation and return the pointer.
/// The host reads this layout from every `search` call return value.
pub fn write_response(data: &[u8]) -> u32 {
    let total = 4 + data.len();
    let mut buf = Vec::with_capacity(total);
    buf.extend_from_slice(&(data.len() as u32).to_le_bytes());
    buf.extend_from_slice(data);
    let ptr = buf.as_ptr() as u32;
    std::mem::forget(buf);
    ptr
}

/// Reconstruct a slice from a pointer+len the host wrote into our memory.
///
/// # Safety
/// Caller must ensure ptr and len describe a valid, initialized region.
pub unsafe fn read_request<'a>(ptr: u32, len: u32) -> &'a [u8] {
    unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize) }
}

/// Macro that emits the `alloc` and `free` exports every plugin must have.
/// Put `plugin_base::export_allocator!()` at the crate root.
#[macro_export]
macro_rules! export_allocator {
    () => {
        #[unsafe(no_mangle)]
        pub extern "C" fn alloc(len: u32) -> *mut u8 {
            let mut buf = Vec::with_capacity(len as usize);
            let ptr = buf.as_mut_ptr();
            std::mem::forget(buf);
            ptr
        }

        #[unsafe(no_mangle)]
        #[allow(clippy::not_unsafe_ptr_arg_deref)]
        pub extern "C" fn free(ptr: *mut u8, len: u32) {
            unsafe { drop(Vec::from_raw_parts(ptr, len as usize, len as usize)) }
        }
    };
}
