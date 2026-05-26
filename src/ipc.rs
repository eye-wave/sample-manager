use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::{Arc, RwLock, mpsc};

use tao::window::Window;

use crate::state::AppState;
use crate::{AStr, LogErrorExt};

mod audio;
mod config;
mod fs;
mod logger;
mod plugins;
mod samples;
mod theme;
mod window;

pub const IPC_ID_BASE: usize = 10;

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct IPCError(Cow<'static, str>);

#[derive(Debug, thiserror::Error)]
#[error("Lock is poisoned.")]
pub struct Poisoned;

impl IPCError {
    pub fn empty() -> Self {
        Self(Cow::Borrowed(""))
    }
}

impl From<&'static str> for IPCError {
    fn from(value: &'static str) -> Self {
        Self(Cow::Borrowed(value))
    }
}

impl From<String> for IPCError {
    fn from(value: String) -> Self {
        Self(Cow::Owned(value))
    }
}

pub fn ok() -> IPCResponse {
    [].finish()
}

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

impl IntoBytes for PathBuf {
    fn into_bytes(self) -> Cow<'static, [u8]> {
        let str = self.to_string_lossy().to_string();
        Cow::Owned(str.into_bytes())
    }
}

pub type IPCRequestBody<'a> = (usize, u32, &'a str);
pub type IPCResponse = Result<std::borrow::Cow<'static, [u8]>, Box<dyn std::error::Error>>;

#[derive(Clone)]
pub struct IPCSenderUI(pub(super) mpsc::Sender<IPCMessage>);

impl IPCSenderUI {
    pub fn send_str(&self, id: &'static str, payload: &'static str) {
        let _ = self.0.send(IPCMessage {
            id,
            payload: Cow::Borrowed(payload),
        });
    }

    pub fn send_msg(&self, id: &'static str, payload: String) {
        let _ = self.0.send(IPCMessage {
            id,
            payload: Cow::Owned(payload),
        });
    }
}

#[derive(Clone)]
pub struct IPCBody {
    pub req: AStr,
    pub window_handle: Arc<Window>,
    pub app_state: Arc<RwLock<AppState>>,
    pub webview_sender: IPCSenderUI,
}

pub struct IPCMessage {
    pub id: &'static str,
    pub payload: Cow<'static, str>,
}

pub trait IPCCommand: Send + Sync {
    #[allow(unused)]
    fn name(&self) -> &'static str;
    fn respond(&self, body: IPCBody) -> IPCResponse;
}

pub trait IntoIPCResponse {
    fn finish(self) -> IPCResponse;
}

impl<T: IntoBytes> IntoIPCResponse for T {
    fn finish(self) -> IPCResponse {
        Ok(self.into_bytes())
    }
}

impl<T: IntoBytes> IntoIPCResponse for Option<T> {
    fn finish(self) -> IPCResponse {
        self.map(|s| s.into_bytes())
            .ok_or(Box::new(IPCError::empty()))
    }
}

pub(super) fn ipc_strip_cmd_id(req: &str) -> Option<IPCRequestBody<'_>> {
    let mut parts = req.splitn(3, ':');

    let fn_name = parts.next()?;
    let id_str = parts.next()?;
    let payload = parts.next().unwrap_or("");

    let id = id_str.parse::<u32>().sure("Failed to parse command id")?;
    Some((
        fn_name.parse().sure("Failed to parse fn_name")?,
        id,
        payload,
    ))
}

pub fn commands_iter<'a>() -> impl Iterator<Item = &'a dyn IPCCommand> {
    use crate::ipc::audio::IPC_AUDIO;
    use crate::ipc::config::IPC_CONFIG;
    use crate::ipc::fs::IPC_FS;
    use crate::ipc::logger::IPC_LOGGER;
    use crate::ipc::plugins::IPC_PLUGINS;
    use crate::ipc::samples::IPC_SAMPLES;
    use crate::ipc::theme::IPC_THEME;
    use crate::ipc::window::IPC_WINDOW;

    IPC_WINDOW
        .iter()
        .chain(IPC_AUDIO.iter())
        .chain(IPC_CONFIG.iter())
        .chain(IPC_FS.iter())
        .chain(IPC_LOGGER.iter())
        .chain(IPC_PLUGINS.iter())
        .chain(IPC_SAMPLES.iter())
        .chain(IPC_THEME.iter())
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
                    ) -> IPCResponse {
                        $fn(body)
                    }
                }
            )*
        }
    };
}

#[macro_export]
macro_rules! with_state {
    ($body:ident, $state:ident, $block:block) => {{
        let $state = $body.app_state.read().map_err(|_| $crate::ipc::Poisoned)?;

        $block
    }};
}

#[macro_export]
macro_rules! with_state_mut {
    ($body:ident, $state:ident, $block:block) => {{
        let mut $state = $body.app_state.write().map_err(|_| $crate::ipc::Poisoned)?;

        $block
    }};
}

#[cfg(test)]
mod test {
    use std::{fs, path::Path};

    use super::*;

    #[test]
    fn generate_ipc() {
        let mut contents: String = commands_iter()
            .enumerate()
            .map(|(i, c)| {
                format!(
                    "export const {} = {};",
                    c.name().to_uppercase(),
                    i + IPC_ID_BASE
                ) + "\n"
            })
            .collect();

        let max_id = commands_iter().count() + IPC_ID_BASE - 1;

        contents += &format!(
            r#"
type Enumerate<
    N extends number,
    Acc extends number[] = []
> = Acc['length'] extends N
    ? Acc[number]
    : Enumerate<N, [...Acc, Acc['length']]>;

type Range<F extends number, T extends number> =
    Exclude<Enumerate<T>, Enumerate<F>> | T;

export type IPC_ID = Range<{}, {}>;"#,
            IPC_ID_BASE, max_id
        );

        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("client/src/gen");

        let _ = fs::create_dir(&path);
        let _ = fs::write(path.join("ipc-gen.ts"), contents);
    }
}
