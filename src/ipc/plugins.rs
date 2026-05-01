use std::{fs, path::PathBuf};

use crate::ipc::{IPCBody, IPCError, IPCResponse, ok};

fn load_plugin(body: IPCBody) -> IPCResponse {
    let id = PathBuf::from(body.req.to_string());
    let id = id
        .file_name()
        .ok_or(IPCError::empty())?
        .to_str()
        .ok_or(IPCError::empty())?;

    let bytes = fs::read(&body.req.as_ref())?;

    crate::with_state!(body, state, {
        state.plugin_handle.load(id, bytes);

        ok()
    })
}

crate::ipc_commands! {
    IPC_PLUGINS = [
        load_plugin
    ]
}
