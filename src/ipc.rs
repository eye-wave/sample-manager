use std::borrow::Cow;
use std::sync::{Arc, RwLock, mpsc};

use tao::window::Window;

use crate::state::AppState;

mod fs;
mod logger;
mod samples;
mod waveform;
mod window;

trait IntoBytes {
    fn into_bytes(self) -> Cow<'static, [u8]>;
}

impl IntoBytes for Vec<u8> {
    fn into_bytes(self) -> Cow<'static, [u8]> {
        Cow::Owned(self)
    }
}

impl IntoBytes for String {
    fn into_bytes(self) -> Cow<'static, [u8]> {
        Cow::Owned(self.into_bytes())
    }
}

impl IntoBytes for &'static str {
    fn into_bytes(self) -> Cow<'static, [u8]> {
        Cow::Borrowed(self.as_bytes())
    }
}

impl IntoBytes for &'static [u8] {
    fn into_bytes(self) -> Cow<'static, [u8]> {
        Cow::Borrowed(self)
    }
}

pub type IPCRequestBody<'a> = (&'a str, u32, &'a str);

#[derive(Clone)]
pub struct IPCBody {
    pub req: Arc<str>,
    pub window_handle: Arc<Window>,
    pub app_state: Arc<RwLock<AppState>>,
    pub webview_sender: mpsc::Sender<IPCMessage>,
}

pub struct IPCMessage {
    pub id: &'static str,
    pub payload: String,
}

pub trait IPCCommand: Send + Sync {
    fn name(&self) -> &'static str;
    fn respond(&self, body: IPCBody) -> Option<Cow<'static, [u8]>>;
}

pub trait IPCResponse {
    fn finish(self) -> Option<Cow<'static, [u8]>>;
}

impl<T: IntoBytes> IPCResponse for T {
    fn finish(self) -> Option<Cow<'static, [u8]>> {
        Some(self.into_bytes())
    }
}

impl<T: IntoBytes> IPCResponse for Option<T> {
    fn finish(self) -> Option<Cow<'static, [u8]>> {
        self.map(|s| s.into_bytes())
    }
}

pub(super) fn ipc_strip_name(req: &str) -> Option<IPCRequestBody<'_>> {
    let mut parts = req.splitn(3, ':');

    let fn_name = parts.next()?;
    let id_str = parts.next()?;
    let payload = parts.next().unwrap_or("");

    let id = id_str.parse::<u32>().ok()?;
    Some((fn_name, id, payload))
}

pub fn commands_iter<'a>() -> impl Iterator<Item = &'a dyn IPCCommand> {
    use crate::ipc::fs::IPC_FS;
    use crate::ipc::logger::IPC_LOGGER;
    use crate::ipc::samples::IPC_SAMPLES;
    use crate::ipc::waveform::IPC_WAVEFORM;
    use crate::ipc::window::IPC_WINDOW;

    IPC_WINDOW
        .iter()
        .chain(IPC_FS.iter())
        .chain(IPC_LOGGER.iter())
        .chain(IPC_SAMPLES.iter())
        .chain(IPC_WAVEFORM.iter())
        .copied()
}

#[macro_export]
macro_rules! ipc_commands {
    (
        $table:ident = [
            $( $fn:ident ),* $(,)?
        ]
    ) => {
        paste::paste! {
            pub(super) static $table: &[&dyn $crate::ipc::IPCCommand] = &[ $( &[<$fn:camel>] ),* ];

            $(
                pub struct [<$fn:camel>];

                impl $crate::ipc::IPCCommand for [<$fn:camel>] {
                    fn name(&self) -> &'static str {
                        stringify!($fn)
                    }

                    fn respond(
                        &self,
                        body: $crate::ipc::IPCBody,
                    ) -> Option<std::borrow::Cow<'static, [u8]>> {
                        $fn(body)
                    }
                }
            )*
        }
    };
}
