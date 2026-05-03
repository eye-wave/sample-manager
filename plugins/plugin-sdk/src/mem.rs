use plugin_wire::{WireEntry, write_frame};

/// Serialize `entries` into a frame buffer, leak it, and return the pointer.
/// The host reads this pointer as the return value of `search` / `get_index`,
/// then calls `free(ptr, frame_size)` once it has parsed the frame.
pub fn write_frame_ptr(entries: &[WireEntry]) -> u32 {
    let buf = write_frame(entries);
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
/// Put `plugin_sdk::export_allocator!()` at the crate root.
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
