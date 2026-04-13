use std::borrow::Cow;
use std::fs;
use std::sync::{Arc, RwLock};

use fuzzy_matcher::skim::SkimMatcherV2;
use rayon::prelude::*;

use crate::commands::IPCResponse;
use crate::ipc_commands;
use crate::state::AppState;
use crate::state::samples::process_directories;

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

/// Start a job that looks over the file system
fn start_sample_scan(
    _r: &str,
    _w: &Arc<tao::window::Window>,
    state: Arc<RwLock<AppState>>,
) -> Option<std::borrow::Cow<'static, [u8]>> {
    let thread_state = state.clone();
    let dirs = state.read().ok()?.get_config().tracked_dirs.clone();

    if dirs.is_empty() {
        return None;
    }

    std::thread::spawn(move || {
        process_directories(dirs, thread_state).ok();
        println!("Finished scan");
    });

    Some(Cow::Borrowed(b"Ok"))
}

/// Search for a sample with a query
pub fn search_for_sample(
    query: &str,
    _w: &Arc<tao::window::Window>,
    state: Arc<RwLock<AppState>>,
) -> Option<std::borrow::Cow<'static, [u8]>> {
    let query = query.to_lowercase();

    let guard = state.read().ok()?;
    let registry = guard.sample_registry.clone();

    let matcher = SkimMatcherV2::default().smart_case();

    let mut scored: Vec<_> = registry
        .par_iter()
        .map(|s| (s, s.score(&query, &matcher)))
        .collect();

    scored.sort_by_key(|a| std::cmp::Reverse(a.1));

    let found = &scored[..10.min(scored.len())];
    let files = found
        .iter()
        .map(|f| f.0.path.to_string_lossy())
        .collect::<Vec<_>>()
        .join("\n");

    files.finish()
}

ipc_commands! {
    IPC_SAMPLES = [
        add_sample_folder,
        get_sample_folders,
        start_sample_scan,
        search_for_sample
    ]
}
