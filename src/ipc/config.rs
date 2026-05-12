use crate::ipc::{IPCBody, IPCResponse, IntoIPCResponse, ok};
use crate::ipc_commands;
use crate::state::config::AppConfigPatch;
use crate::state::samples::process_directories;

fn patch_config(body: IPCBody) -> IPCResponse {
    let mut is_new = false;
    let dirs = crate::with_state_mut!(body, state, {
        let patch = {
            let mut patch: AppConfigPatch = serde_json::from_str(&body.req)?;

            if let Some(td) = patch.tracked_dirs.as_mut() {
                td.retain(|d| d.exists());
                is_new = td
                    .iter()
                    .any(|d| !state.get_config().tracked_dirs.contains(d))
                    && !td.is_empty();
            }

            state.patch_config(patch.clone());
            patch
        };

        patch.tracked_dirs.clone()
    })
    .unwrap_or_default();

    if is_new {
        let thread_state = body.app_state.clone();
        std::thread::spawn(move || {
            process_directories(dirs.iter(), thread_state, body.webview_sender).ok();
        });
    }

    ok()
}

fn get_config_fields(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        serde_json::to_string(&state.get_config().as_fields())?.finish()
    })
}

fn get_config_field(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        state.get_config().get_field_as_json(&body.req)?.finish()
    })
}

ipc_commands! {
    IPC_CONFIG = [
        patch_config,
        get_config_fields,
        get_config_field
    ]
}
