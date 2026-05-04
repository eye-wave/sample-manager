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

use paste::paste;

macro_rules! impl_storage_num {
    ($t:ty, $len:expr) => {
        paste! {
            pub fn [<storage_get_ $t>](key: &str) -> Option<$t> {
                storage_get(key).and_then(|b| {
                    if b.len() != $len {
                        return None;
                    }
                    let arr: [u8; $len] = b.try_into().ok()?;
                    Some(<$t>::from_le_bytes(arr))
                })
            }

            pub fn [<storage_set_ $t>](key: &str, value: $t) {
                storage_set(key, &value.to_le_bytes());
            }

            pub fn [<secret_get_ $t>](key: &str) -> Option<$t> {
                secret_get(key).and_then(|b| {
                    if b.len() != $len {
                        return None;
                    }
                    let arr: [u8; $len] = b.try_into().ok()?;
                    Some(<$t>::from_le_bytes(arr))
                })
            }

            pub fn [<secret_set_ $t>](key: &str, value: $t) {
                secret_set(key, &value.to_le_bytes());
            }
        }
    };
}

impl_storage_num!(u8, 1);
impl_storage_num!(u16, 2);
impl_storage_num!(u32, 4);
impl_storage_num!(u64, 8);

impl_storage_num!(i8, 1);
impl_storage_num!(i16, 2);
impl_storage_num!(i32, 4);
impl_storage_num!(i64, 8);

impl_storage_num!(f32, 4);
impl_storage_num!(f64, 8);

pub fn storage_get_string(key: &str) -> Option<String> {
    storage_get(key).and_then(|b| String::from_utf8(b).ok())
}

pub fn storage_set_string(key: &str, value: &str) {
    storage_set(key, value.as_bytes());
}
