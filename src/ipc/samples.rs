use std::fs;

use crate::ipc::{IPCBody, IPCError, IPCResponse, IntoIPCResponse, Poisoned, ok};
use crate::state::FsSample;
use crate::state::samples::{
    SampleEntry, ScanMerge, SearchRequest, WaveformData, draw_audio_and_save, process_directories,
    search_local, tag_string,
};
use crate::{LogErrorExt, ipc_commands};

fn add_sample_folder(body: IPCBody) -> IPCResponse {
    let path = body.req.as_ref();

    if fs::read_dir(path).is_err() {
        Err(IPCError::from("Path is empty"))?;
    }

    crate::with_state_mut!(body, state, {
        state.update_config(|cfg| {
            cfg.tracked_dirs.insert(path.into());
        });

        b"Ok".finish()
    })
}

fn get_sample_folders(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        state
            .get_config()
            .tracked_dirs
            .iter()
            .map(|d| d.to_string_lossy().to_string() + "\n")
            .collect::<String>()
            .finish()
    })
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
        process_directories(
            dirs.iter(),
            ScanMerge::ReplaceUnder(dirs.clone()),
            thread_state,
            body.webview_sender,
        )
        .sure("Failed to process directories");
    });

    ok()
}

fn search_for_sample(body: IPCBody) -> IPCResponse {
    let req: SearchRequest = serde_json::from_str(&body.req)?;

    std::thread::spawn(move || {
        let state = body.app_state.read().unwrap();
        if let Ok(payload) = search_local(&req, &state) {
            body.webview_sender.send_msg("search", payload);
        }
    });

    ok()
}

fn get_sample_metadata(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        let sample = FsSample::new(body.req.as_ref());

        sample.to_json(&state)?.finish()
    })
}

const SET_FAV_ID: &str = "set-fav";

fn toggle_sample_fav(body: IPCBody) -> IPCResponse {
    crate::with_state_mut!(body, state, {
        let is_fav = state.favorite_samples.contains(body.req.as_ref());

        if is_fav {
            state.remove_sample_from_fav(&body.req);
        } else {
            state.add_sample_to_fav(&body.req);
        }

        let prefix = if is_fav { "0" } else { "1" };
        body.webview_sender
            .send_msg(SET_FAV_ID, prefix.to_string() + &body.req);

        ok()
    })
}

fn is_sample_fav(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        (state.favorite_samples.contains(body.req.as_ref())).finish()
    })
}

fn tag_path(body: IPCBody) -> IPCResponse {
    tag_string(&body.req)
        .iter()
        .cloned()
        .intersperse(",")
        .collect::<String>()
        .finish()
}

fn draw_audio_file(body: IPCBody) -> IPCResponse {
    let ffpaths = crate::with_state!(body, state, { state.get_config().ffpaths.clone() });

    std::thread::spawn(move || {
        let path = body.req.clone();

        match draw_audio_and_save(None, &path, WaveformData::Path(&path), ffpaths.flatten()) {
            Ok(msg) => {
                msg.send_to_webview(body.webview_sender);
            }
            Err(err) => tracing::error!(
                error = %err,
                "draw_audio_file failed"
            ),
        }
    });

    ok()
}

ipc_commands! {
    IPC_SAMPLES = [
        toggle_sample_fav,
        is_sample_fav,
        add_sample_folder,
        get_sample_folders,
        get_sample_metadata,
        start_sample_scan,
        search_for_sample,
        tag_path,
        draw_audio_file
    ]
}
