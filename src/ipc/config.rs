use crate::{
    ipc::{IPCBody, IPCResponse, IntoIPCResponse, ok},
    ipc_commands,
    state::config::ConfigField,
};

fn update_config_field(body: IPCBody) -> IPCResponse {
    crate::with_state_mut!(body, state, {
        let field: ConfigField = serde_json::from_str(&body.req)?;

        state.mutate_config_field(field);

        ok()
    })
}

fn get_settings(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        serde_json::to_string(state.get_config())?.finish()
    })
}

ipc_commands! {
    IPC_CONFIG = [
        update_config_field,
        get_settings
    ]
}
