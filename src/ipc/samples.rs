use std::fs;

use fuzzy_matcher::skim::SkimMatcherV2;
use rayon::prelude::*;

use crate::ipc::{IPCBody, IPCError, IPCResponse, IntoIPCResponse, Poisoned, ok};
use crate::ipc_commands;
use crate::state::samples::{process_directories, tag_string};

fn add_sample_folder(body: IPCBody) -> IPCResponse {
    let path = body.req.as_ref();

    if fs::read_dir(path).is_err() {
        Err(IPCError::from("Path is empty"))?;
    }

    let mut guard = body.app_state.write().map_err(|_| Poisoned)?;
    guard.update_config(|cfg| {
        cfg.tracked_dirs.insert(path.into());
    });

    ok()
}

fn get_sample_folders(body: IPCBody) -> IPCResponse {
    let guard = body.app_state.read().map_err(|_| Poisoned)?;
    let cfg = guard.get_config();

    cfg.tracked_dirs
        .iter()
        .map(|d| d.to_string_lossy().to_string() + "\n")
        .collect::<String>()
        .finish()
}

fn start_sample_scan(body: IPCBody) -> IPCResponse {
    let thread_state = body.app_state.clone();
    let guard = body.app_state.read().map_err(|_| Poisoned)?;
    let dirs = guard.get_config().tracked_dirs.clone();

    if dirs.is_empty() {
        Err(IPCError::from("Path is empty"))?;
    }

    std::thread::spawn(move || {
        process_directories(dirs.iter(), thread_state, body.webview_sender).ok();
    });

    ok()
}

fn search_for_sample(body: IPCBody) -> IPCResponse {
    let tokens = body.req.to_lowercase();
    let tokens = tokens.split(",").map(|t| t.trim()).collect::<Vec<_>>();

    let (tags, query) = tokens.split_at(tokens.len().saturating_sub(1));

    let query = query.first().unwrap_or(&"");

    let guard = body.app_state.read().map_err(|_| Poisoned)?;
    let registry = guard.sample_registry.clone();

    let matcher = SkimMatcherV2::default().smart_case();

    let mut scored: Vec<_> = registry
        .par_iter()
        .map(|s| (s, s.score(query, tags, &matcher)))
        .collect();

    scored.sort_by_key(|&(_, score)| std::cmp::Reverse(score));

    let found = &scored[..100.min(scored.len())];
    let files = found
        .iter()
        .map(|(f, _)| f.serialize())
        .intersperse(",\n".into())
        .collect::<String>();

    format!("[{files}]").finish()
}

fn tag_path(body: IPCBody) -> IPCResponse {
    tag_string(&body.req)
        .iter()
        .cloned()
        .intersperse(",")
        .collect::<String>()
        .finish()
}

ipc_commands! {
    IPC_SAMPLES = [
        add_sample_folder,
        get_sample_folders,
        start_sample_scan,
        search_for_sample,
        tag_path
    ]
}
