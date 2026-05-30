use std::sync::mpsc;
use std::{borrow::Cow, path::PathBuf};

use serde::Serialize;

use crate::LogErrorExt;

mod commands;
mod state;

pub use commands::commands_iter;
pub(super) use state::IPCBody;

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

pub const fn ok() -> IPCResponse {
    IPCResponse::Ok(Cow::Borrowed(""))
}

pub type IPCRequestBody<'a> = (usize, u32, &'a str);
pub type IPCResponse = Result<std::borrow::Cow<'static, str>, Box<dyn std::error::Error>>;

#[derive(Clone)]
pub struct IPCSenderUI(pub(super) mpsc::Sender<IPCMessage>);

type Id = &'static str;
impl IPCSenderUI {
    pub fn send_ping(&self, id: Id) {
        let _ = self.0.send(IPCMessage {
            id,
            payload: Cow::Borrowed(""),
        });
    }

    pub fn send_str(&self, id: Id, payload: &'static str) {
        let _ = self.0.send(IPCMessage {
            id,
            payload: Cow::Borrowed(payload),
        });
    }

    pub fn send_msg(&self, id: Id, payload: impl ToString) {
        let _ = self.0.send(IPCMessage {
            id,
            payload: Cow::Owned(payload.to_string()),
        });
    }

    pub fn send_json(&self, id: Id, blob: &impl Serialize) {
        if let Ok(msg) = serde_json::to_string(blob) {
            self.send_msg(id, msg);
        }
    }
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

impl IntoIPCResponse for String {
    fn finish(self) -> IPCResponse {
        Ok(Cow::Owned(self))
    }
}

impl IntoIPCResponse for PathBuf {
    fn finish(self) -> IPCResponse {
        Ok(Cow::Owned(self.to_string_lossy().to_string()))
    }
}

impl IntoIPCResponse for &'static str {
    fn finish(self) -> IPCResponse {
        Ok(Cow::Borrowed(self))
    }
}

impl IntoIPCResponse for bool {
    fn finish(self) -> IPCResponse {
        Ok(Cow::Borrowed(if self { "1" } else { "0" }))
    }
}

macro_rules! impl_numeric_ipc {
    ($($t:ty),*) => {
        $(
            impl IntoIPCResponse for $t {
                fn finish(self) -> IPCResponse {
                    Ok(Cow::Owned(self.to_string()))
                }
            }
        )*
    };
}

impl_numeric_ipc!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64
);

pub trait IntoIPCJsonResponse {
    fn finish_json(&self) -> IPCResponse;
}
impl<T: Serialize> IntoIPCJsonResponse for T {
    fn finish_json(&self) -> IPCResponse {
        Ok(Cow::Owned(serde_json::to_string(&self)?))
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
