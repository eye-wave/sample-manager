use crate::{
    ipc::{IPCBody, IPCResponse, IntoIPCResponse, ok},
    ipc_commands,
    state::config::{AppConfigPatch, ConfigField},
};

fn patch_config(body: IPCBody) -> IPCResponse {
    crate::with_state_mut!(body, state, {
        let patch: AppConfigPatch = serde_json::from_str(&body.req)?;
        println!("received patch: {patch:?}");

        state.mutate_config_field(patch);

        ok()
    })
}

fn get_config_as_json(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        serde_json::to_string(state.get_config())?.finish()
    })
}

fn get_config_field(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        let field: ConfigField = serde_json::from_str(&format!("\"{}\"", body.req))?;
        state.get_config().get_field_as_json(field)?.finish()
    })
}

ipc_commands! {
    IPC_CONFIG = [
        patch_config,
        get_config_as_json,
        get_config_field
    ]
}
