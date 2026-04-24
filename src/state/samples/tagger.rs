use std::ffi::{CStr, CString, c_char, c_ushort};

unsafe extern "C" {
    fn search(text: *const c_char, buffer: *const *const c_char, max_out: c_ushort);
}

pub fn tag_string(text: &str) -> Vec<&'static str> {
    const MAX_SIZE: usize = 20;

    let input = CString::new(text.to_lowercase()).unwrap();
    let mut buffer: [*const c_char; MAX_SIZE] = [std::ptr::null(); MAX_SIZE];

    unsafe {
        search(input.as_ptr(), buffer.as_mut_ptr(), MAX_SIZE as u16);
    }

    buffer
        .iter()
        .filter(|p| !p.is_null())
        .map(|ptr| unsafe { CStr::from_ptr(*ptr).to_str().unwrap() })
        .collect()
}
