use std::borrow::Cow;
use std::sync::{Arc, RwLock, mpsc};

use crate::event::IPCMessage;
use crate::state::AppState;

mod bytes;

#[derive(Clone)]
pub struct IPCBody {
    pub req: Arc<str>,
    pub window_handle: Arc<tao::window::Window>,
    pub app_state: Arc<RwLock<AppState>>,
    pub webview_sender: mpsc::Sender<IPCMessage>,
}

pub trait IPCCommand: Send + Sync {
    fn name(&self) -> &'static str;
    fn respond(&self, body: IPCBody) -> Option<Cow<'static, [u8]>>;
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
