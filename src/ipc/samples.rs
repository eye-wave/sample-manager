use std::fs;
use std::sync::{Arc, RwLock};

use crate::commands::IPCResponse;
use crate::ipc_commands;
use crate::state::AppState;

/// Adds a sample folder to app state
fn add_sample_folder(
    path: &str,
    _w: &Arc<tao::window::Window>,
    state: Arc<RwLock<AppState>>,
) -> Option<std::borrow::Cow<'static, [u8]>> {
    if fs::read_dir(path).is_err() {
        return None;
    }

    let mut guard = state.write().ok()?;
    guard.update_config(|cfg| {
        cfg.tracked_dirs.insert(path.into());
    });

    b"Ok".finish()
}

/// Returns folders with samples added to app state
fn get_sample_folders(
    _r: &str,
    _w: &Arc<tao::window::Window>,
    state: Arc<RwLock<AppState>>,
) -> Option<std::borrow::Cow<'static, [u8]>> {
    let guard = state.read().ok()?;
    let cfg = guard.get_config();

    cfg.tracked_dirs
        .iter()
        .map(|d| d.to_string_lossy().to_string() + "\n")
        .collect::<String>()
        .finish()
}

ipc_commands! {
    IPC_SAMPLES = [
        add_sample_folder,
        get_sample_folders
    ]
}
