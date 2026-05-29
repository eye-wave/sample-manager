use crate::ipc::{IPCBody, IPCResponse, IntoIPCJsonResponse, IntoIPCResponse, ok};
use crate::state::config::{AppCacheSize, ConfigDataPatch};
use crate::state::samples::{ScanMerge, process_directories};
use crate::{LogErrorExt, ipc_commands};

fn get_app_cache_size(_: IPCBody) -> IPCResponse {
    AppCacheSize::read().finish_json()
}

fn patch_config(body: IPCBody) -> IPCResponse {
    let mut is_new = false;
    let dirs = {
        let patch = {
            let mut state = body.write_state()?;
            let mut patch: ConfigDataPatch = body.parse_req()?;

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
    }
    .unwrap_or_default();

    if is_new {
        let thread_state = body.clone_state_lock();
        std::thread::spawn(move || {
            process_directories(
                dirs.iter(),
                ScanMerge::ReplaceAll,
                thread_state,
                body.webview_sender,
            )
            .sure("Failed to process directories");
        });
    }

    ok()
}

fn get_config_fields(body: IPCBody) -> IPCResponse {
    let state = body.read_state()?;
    state.get_config().as_fields().finish_json()
}

fn get_config_field(body: IPCBody) -> IPCResponse {
    let state = body.read_state()?;
    state.get_config().get_field_as_json(&body.req)?.finish()
}

ipc_commands! {
    IPC_CONFIG = [
        get_app_cache_size,
        patch_config,
        get_config_fields,
        get_config_field
    ]
}
