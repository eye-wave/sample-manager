use std::sync::{Arc, RwLock};

use tao::window::Window;

use crate::AStr;
use crate::ipc::{IPCSenderUI, Poisoned};
use crate::state::AppState;

#[derive(Clone)]
pub struct IPCBody {
    pub(self) app_state: Arc<RwLock<AppState>>,
    pub req: AStr,
    pub window_handle: Arc<Window>,
    pub webview_sender: IPCSenderUI,
}

impl IPCBody {
    pub fn new(
        app_state: Arc<RwLock<AppState>>,
        req: AStr,
        window_handle: Arc<Window>,
        webview_sender: IPCSenderUI,
    ) -> Self {
        Self {
            webview_sender,
            req,
            window_handle,
            app_state,
        }
    }

    pub fn clone_state_lock(&self) -> Arc<RwLock<AppState>> {
        self.app_state.clone()
    }

    pub fn read_state(&self) -> Result<std::sync::RwLockReadGuard<'_, AppState>, Poisoned> {
        self.app_state.read().map_err(|_| Poisoned)
    }

    pub fn write_state(&self) -> Result<std::sync::RwLockWriteGuard<'_, AppState>, Poisoned> {
        self.app_state.write().map_err(|_| Poisoned)
    }

    pub fn parse_req<'a, T: serde::Deserialize<'a>>(&'a self) -> Result<T, serde_json::Error> {
        serde_json::from_str(&self.req)
    }
}
