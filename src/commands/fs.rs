use std::borrow::Cow;
use std::sync::{Arc, RwLock};

use crate::ipc_commands;
use crate::state::AppState;

fn open_folder(
    _r: &str,
    _w: &Arc<tao::window::Window>,
    _s: Arc<RwLock<AppState>>,
) -> Option<std::borrow::Cow<'static, [u8]>> {
    let folder = tinyfiledialogs::select_folder_dialog("Select folder", "");

    folder.map(|f| Cow::Owned(f.into_bytes()))
}

fn search_path(
    path: &str,
    _w: &Arc<tao::window::Window>,
    _s: Arc<RwLock<AppState>>,
) -> Option<std::borrow::Cow<'static, [u8]>> {
    let files: Vec<_> = std::fs::read_dir(path)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|e| {
            let is_dir = e.path().is_dir() as u8;

            e.path()
                .strip_prefix(path)
                .ok()
                .map(|p| is_dir.to_string() + &p.display().to_string())
        })
        .collect();

    if files.is_empty() {
        None
    } else {
        Some(Cow::Owned(files.join("\n").into_bytes()))
    }
}

fn can_read_dir(path: &str) -> bool {
    std::fs::read_dir(path).is_ok()
}

fn add_sample_folder(
    path: &str,
    _w: &Arc<tao::window::Window>,
    state: Arc<RwLock<AppState>>,
) -> Option<std::borrow::Cow<'static, [u8]>> {
    if can_read_dir(path) {
        return None;
    }

    let mut guard = state.write().ok()?;
    guard.update_config(|cfg| {
        cfg.tracked_dirs.insert(path.into());
    });

    None
}

fn get_sample_folders(
    _r: &str,
    _w: &Arc<tao::window::Window>,
    state: Arc<RwLock<AppState>>,
) -> Option<std::borrow::Cow<'static, [u8]>> {
    let guard = state.read().ok()?;
    let cfg = guard.get_config();

    let content = cfg
        .tracked_dirs
        .iter()
        .map(|d| d.to_string_lossy().to_string() + "\n")
        .collect::<String>();

    Some(Cow::Owned(content.into_bytes()))
}

ipc_commands! {
    IPC_FS = [
        search_path,
        open_folder,
        add_sample_folder,
        get_sample_folders
    ]
}
