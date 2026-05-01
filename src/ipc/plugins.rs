use crate::ipc::{IPCBody, IPCError, IPCResponse, IntoIPCResponse};

fn disable_plugin(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        let id = body.req;
        state.plugin_handle.unload(id);

        todo!()
    })
}

fn get_plugin_manifest(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        let id = body.req;
        let manifest = state
            .plugin_handle
            .get_manifest(&id)
            .ok_or(IPCError::empty())?;

        serde_json::to_string(&manifest)?.finish()
    })
}

fn list_all_plugin_ids(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        state
            .plugin_handle
            .list_all_ids()
            .iter()
            .map(|c| c.to_string())
            .intersperse(",".into())
            .collect::<String>()
            .finish()
    })
}

crate::ipc_commands! {
    IPC_PLUGINS = [
        disable_plugin,
        get_plugin_manifest,
        list_all_plugin_ids
    ]
}
