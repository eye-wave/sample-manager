use std::fs;
use std::path::{Path, PathBuf};

use fuzzy_matcher::skim::SkimMatcherV2;
use rayon::iter::Either;
use rayon::prelude::*;

use crate::ipc::{IPCBody, IPCError, IPCResponse, IntoIPCResponse, Poisoned, ok};
use crate::ipc_commands;
use crate::state::samples::{clean_up_string, process_directories, tag_string};

fn add_sample_folder(body: IPCBody) -> IPCResponse {
    let path = body.req.as_ref();

    if fs::read_dir(path).is_err() {
        Err(IPCError::from("Path is empty"))?;
    }

    let mut guard = body.app_state.write().map_err(|_| Poisoned)?;
    guard.update_config(|cfg| {
        cfg.tracked_dirs.insert(path.into());
    });

    b"Ok".finish()
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
    let path = body.req;

    let thread_state = body.app_state.clone();

    let dirs = {
        if path.is_empty() {
            let guard = body.app_state.read().map_err(|_| Poisoned)?;
            guard.get_config().tracked_dirs.iter().cloned().collect()
        } else {
            vec![path.to_string().into()]
        }
    };

    if dirs.is_empty() {
        Err(IPCError::from("Path is empty"))?;
    }

    std::thread::spawn(move || {
        process_directories(dirs.iter(), thread_state, body.webview_sender).ok();
    });

    ok()
}

fn search_for_sample(body: IPCBody) -> IPCResponse {
    let mut limit = 50;
    let mut offset = 0;
    let mut query = None;
    let mut tags = Vec::new();
    let mut is_fav = false;

    for (k, v) in url::form_urlencoded::parse(body.req.as_bytes()) {
        match k.as_ref() {
            "lim" => {
                limit = v.parse().unwrap_or(50);
            }
            "off" => {
                offset = v.parse().unwrap_or(0);
            }
            "q" => {
                query = Some(clean_up_string(&v));
            }
            "fav" => {
                is_fav = v == "1";
            }
            "t" => {
                if !v.is_empty() {
                    tags = v.split(',').map(|x| x.to_string()).collect();
                }
            }
            _ => {}
        }
    }

    let query = query.ok_or(IPCError::empty())?;

    let guard = body.app_state.read().map_err(|_| Poisoned)?;

    let matcher = SkimMatcherV2::default().smart_case();

    let scored = if is_fav {
        Either::Left(guard.favorite_samples.iter().filter_map(|f| {
            let key = Path::new(f);
            guard.sample_registry.get(key)
        }))
    } else {
        Either::Right(guard.sample_registry.values())
    };

    let mut result = scored
        .par_bridge()
        .map(|s| {
            if is_fav && query.is_empty() {
                return (s, i64::MAX);
            }

            let score = s.score(&query, &tags, &matcher);
            (s, score)
        })
        .filter(|(_, score)| *score > 0)
        .collect::<Vec<_>>();

    result.sort_by_key(|&(_, score)| std::cmp::Reverse(score));

    let start = offset;
    let end = (start + limit).min(result.len());

    let found = if start < result.len() {
        &result[start..end]
    } else {
        &[]
    };

    let files = found
        .iter()
        .map(|(f, _)| f.serialize(guard.is_sample_fav(&f.path)))
        .intersperse(",\n".into())
        .collect::<String>();

    format!("{{\"count\":{},\"files\":[{files}]}}", result.len()).finish()
}

const SET_FAV_ID: &str = "set-fav";

fn add_sample_to_fav(body: IPCBody) -> IPCResponse {
    let mut guard = body.app_state.write().map_err(|_| Poisoned)?;
    guard.add_sample_to_fav(body.req.to_string().into());

    body.webview_sender
        .send(super::IPCMessage {
            id: SET_FAV_ID,
            payload: "1".to_string() + &body.req,
        })
        .ok();

    ok()
}

fn remove_sample_from_fav(body: IPCBody) -> IPCResponse {
    let mut guard = body.app_state.write().map_err(|_| Poisoned)?;
    guard.remove_sample_from_fav(body.req.to_string().as_ref());

    body.webview_sender
        .send(super::IPCMessage {
            id: SET_FAV_ID,
            payload: "0".to_string() + &body.req,
        })
        .ok();

    ok()
}

fn is_sample_fav(body: IPCBody) -> IPCResponse {
    let guard = body.app_state.read().map_err(|_| Poisoned)?;

    guard
        .favorite_samples
        .contains(&PathBuf::from(body.req.to_string()))
        .to_string()
        .finish()
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
        add_sample_to_fav,
        remove_sample_from_fav,
        is_sample_fav,
        add_sample_folder,
        get_sample_folders,
        start_sample_scan,
        search_for_sample,
        tag_path
    ]
}
