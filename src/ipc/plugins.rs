use crate::{
    ipc::{IPCBody, IPCResponse, IntoIPCResponse},
    plugins::PluginId,
};

fn disable_plugin(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        let id = PluginId::new(body.req)?;

        state.plugin_handle.unload(id);

        todo!()
    })
}

fn get_all_plugins_info(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        let mut plugins_info = state.loaded_plugins_info.clone();
        let loaded_plugins_info = state.plugin_handle.get_all_plugins_info();

        for p in plugins_info.iter_mut() {
            if loaded_plugins_info
                .iter()
                .find(|pl| pl.meta.id == p.meta.id)
                .is_none()
            {
                p.is_enabled = false
            }
        }

        serde_json::to_string(&plugins_info)?.finish()
    })
}

crate::ipc_commands! {
    IPC_PLUGINS = [
        disable_plugin,
        get_all_plugins_info,
    ]
}
