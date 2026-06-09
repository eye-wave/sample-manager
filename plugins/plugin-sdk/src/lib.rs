mod host;

pub mod download;
pub mod fs;
pub mod http;
pub mod mem;
pub mod storage;

pub mod plugin_wire {
    pub use sample_model::wire::*;
}

pub mod prelude {
    pub use super::download::*;
    pub use super::fs::*;
    pub use super::http::*;
    pub use super::mem::*;
    pub use super::plugin_wire::*;
    pub use super::storage::*;
}

pub fn log(msg: &str) {
    unsafe { host::log(msg.as_ptr(), msg.len() as u32) }
}
