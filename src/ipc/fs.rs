use std::path::Path;
use std::sync::{Arc, RwLock};

use crate::commands::IPCResponse;
use crate::ipc_commands;
use crate::state::AppState;

/// Opens OS folder dialog
fn open_folder(
    _r: &str,
    _w: &Arc<tao::window::Window>,
    _s: Arc<RwLock<AppState>>,
) -> Option<std::borrow::Cow<'static, [u8]>> {
    let folder = tinyfiledialogs::select_folder_dialog("Select folder", "");

    folder.finish()
}

fn get_path_type(path: &Path) -> u8 {
    if path.is_dir() {
        return 0;
    }

    const AUDIO_SAMPLE_DATA: &[&str] = &[
        "aac", "aiff", "caf", "flac", "mp2", "mp3", "mp4", "mpeg", "ogg", "opus", "wav", "wv",
    ];

    const AUDIO_MIDI_DATA: &[&str] = &["mid", "midi"];
    const PLUGIN_PRESETS: &[&str] = &["fxb", "fxp", "vital"];

    let ext = match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => ext,
        None => return 90,
    };

    let contains = |list: &[&str]| list.binary_search(&ext).is_ok();

    if contains(AUDIO_SAMPLE_DATA) {
        return 1;
    }

    if contains(AUDIO_MIDI_DATA) {
        return 2;
    }

    if contains(PLUGIN_PRESETS) {
        return 4;
    }

    90
}

/// Reads items inside a directory
fn read_dir(
    path: &str,
    _w: &Arc<tao::window::Window>,
    _s: Arc<RwLock<AppState>>,
) -> Option<std::borrow::Cow<'static, [u8]>> {
    let mut files: Vec<_> = std::fs::read_dir(path)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|e| {
            const BYTE_OFFSET: u8 = 32;

            let item_type = get_path_type(&e.path()) + BYTE_OFFSET;
            let item_type = unsafe { String::from_utf8_unchecked(vec![item_type]) };

            if let Ok(p) = e.path().strip_prefix(path) {
                Some(item_type + &p.display().to_string())
            } else {
                None
            }
        })
        .collect();

    files.sort();

    if files.is_empty() {
        None
    } else {
        files.join("\n").finish()
    }
}

fn can_read_dir(path: &str) -> bool {
    std::fs::read_dir(path).is_ok()
}

/// Adds a sample folder to app state
fn add_sample_folder(
    path: &str,
    _w: &Arc<tao::window::Window>,
    state: Arc<RwLock<AppState>>,
) -> Option<std::borrow::Cow<'static, [u8]>> {
    if !can_read_dir(path) {
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
    IPC_FS = [
        read_dir,
        open_folder,
        add_sample_folder,
        get_sample_folders
    ]
}
