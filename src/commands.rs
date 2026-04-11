use std::borrow::Cow;
use std::sync::{Arc, RwLock};

use crate::state::AppState;

mod bytes;

pub trait IPCCommand: Send + Sync {
    fn name(&self) -> &'static str;
    fn respond(
        &self,
        req: &str,
        window_handle: &Arc<tao::window::Window>,
        state: Arc<RwLock<AppState>>,
    ) -> Option<Cow<'static, [u8]>>;
}

pub type IPCRequestBody<'a> = (&'a str, u32, &'a str);

pub trait IPCResponse {
    fn finish(self) -> Option<Cow<'static, [u8]>>;
}

impl<T: bytes::IntoBytes> IPCResponse for T {
    fn finish(self) -> Option<Cow<'static, [u8]>> {
        Some(self.into_bytes())
    }
}

impl<T: bytes::IntoBytes> IPCResponse for Option<T> {
    fn finish(self) -> Option<Cow<'static, [u8]>> {
        self.map(|s| s.into_bytes())
    }
}
