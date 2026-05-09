use crate::ipc::{IPCBody, IPCResponse, IntoIPCResponse, ok};
use crate::ipc_commands;
use crate::state::config::AppConfigPatch;

fn patch_config(body: IPCBody) -> IPCResponse {
    crate::with_state_mut!(body, state, {
        let patch: AppConfigPatch = serde_json::from_str(&body.req)?;
        state.patch_config(patch);

        ok()
    })
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
