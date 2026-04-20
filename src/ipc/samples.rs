use std::borrow::Cow;
use std::fs;

use fuzzy_matcher::skim::SkimMatcherV2;
use rayon::prelude::*;

use crate::ipc::{IPCBody, IPCResponse};
use crate::ipc_commands;
use crate::state::samples::process_directories;

fn add_sample_folder(body: IPCBody) -> Option<Cow<'static, [u8]>> {
    let path = body.req.as_ref();

    if fs::read_dir(path).is_err() {
        return None;
    }

    let mut guard = body.app_state.write().ok()?;
    guard.update_config(|cfg| {
        cfg.tracked_dirs.insert(path.into());
    });

    b"Ok".finish()
}

fn get_sample_folders(body: IPCBody) -> Option<Cow<'static, [u8]>> {
    let guard = body.app_state.read().ok()?;
    let cfg = guard.get_config();

    cfg.tracked_dirs
        .iter()
        .map(|d| d.to_string_lossy().to_string() + "\n")
        .collect::<String>()
        .finish()
}

fn start_sample_scan(body: IPCBody) -> Option<Cow<'static, [u8]>> {
    let thread_state = body.app_state.clone();
    let guard = body.app_state.read().ok()?;
    let dirs = guard.get_config().tracked_dirs.clone();

    if dirs.is_empty() {
        return None;
    }

    std::thread::spawn(move || {
        process_directories(dirs, thread_state, body.webview_sender).ok();
    });

    Some(Cow::Borrowed(b"Ok"))
}

fn search_for_sample(body: IPCBody) -> Option<Cow<'static, [u8]>> {
    let tokens = body.req.to_lowercase();
    let tokens = tokens.split(",").map(|t| t.trim()).collect::<Vec<_>>();

    let (tags, query) = tokens.split_at(tokens.len().saturating_sub(1));

    let query = query.first().unwrap_or(&"");

    let guard = body.app_state.read().ok()?;
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

ipc_commands! {
    IPC_SAMPLES = [
        add_sample_folder,
        get_sample_folders,
        start_sample_scan,
        search_for_sample
    ]
}
