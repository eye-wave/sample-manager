use crate::ipc::{IPCBody, IPCResponse, IntoIPCResponse};

fn disable_plugin(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        let id = body.req;
        state.plugin_handle.unload(id);

        todo!()
    })
}

fn get_all_plugins_info(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        let plugin_info_list = state.plugin_handle.get_all_plugins_info();

        serde_json::to_string(&plugin_info_list)?.finish()
    })
}

crate::ipc_commands! {
    IPC_PLUGINS = [
        disable_plugin,
        get_all_plugins_info,
    ]
}
