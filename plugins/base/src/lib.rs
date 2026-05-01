mod host;

pub mod http;
pub mod json;
pub mod mem;
pub mod storage;
pub mod types;

pub use json::JsonValue;
pub use types::{SampleResult, SearchRequest};

pub mod prelude {
    pub use super::http::*;
    pub use super::json::*;
    pub use super::mem::*;
    pub use super::storage::*;
    pub use super::types::*;
}

pub fn log(msg: &str) {
    unsafe { host::log(msg.as_ptr(), msg.len() as u32) }
}
