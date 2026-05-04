use plugin_wire::sample::SampleEntryBase;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    AStr,
    ipc::{IPCBody, IPCMessage, IPCResponse, IntoIPCResponse, ok},
    plugins::{PluginId, parse_string_to_bytes},
    state::samples::SearchRequest,
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

#[derive(Deserialize, TS)]
#[ts(export)]
struct ConfigPluginValueUpdate {
    id: PluginId,
    name: AStr,
    data: String,
}

fn configure_plugin_value(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        let ConfigPluginValueUpdate { id, name, data } = serde_json::from_str(&body.req)?;

        let data = parse_string_to_bytes(data);

        state.plugin_handle.set_config_field(id, name, data);

        ok()
    })
}

#[derive(Serialize)]
struct SearchResult {
    files: Vec<SampleEntryBase>,
    count: usize,
}

fn plugin_search_for_sample(body: IPCBody) -> IPCResponse {
    std::thread::spawn(
        move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let req: SearchRequest = serde_json::from_str(&body.req)?;
            let id = PluginId::new("plugin-id").unwrap();

            let state = body.app_state.read().unwrap();
            let files = state.plugin_handle.search(id, req)?;
            let count = files.len();
            let res = SearchResult { files, count };

            let _ = body.webview_sender.send(IPCMessage {
                id: "search",
                payload: serde_json::to_string(&res)?,
            });

            Ok(())
        },
    );

    ok()
}

fn plugin_download_file(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        // let id = PluginId::new("plugin-id").unwrap();
        let _ = state.plugin_handle.download(id, &body.req);
    });

    ok()
}

crate::ipc_commands! {
    IPC_PLUGINS = [
        disable_plugin,
        get_all_plugins_info,
        configure_plugin_value,
        plugin_search_for_sample,
        plugin_download_file
    ]
}
