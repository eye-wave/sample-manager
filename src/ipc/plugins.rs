use std::fs;
use std::path::{Path, PathBuf};

use plugin_wire::sample::SampleSerialize;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::ipc::{IPCBody, IPCError, IPCResponse, IntoIPCResponse, ok};
use crate::plugins::PluginId;
use crate::state::{app_paths, samples::SearchRequest};
use crate::{AStr, LogErrorExt};

fn any_online_plugin_loaded(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        ((state
            .plugin_handle
            .get_all_plugins_info(|_| {})
            .iter()
            .filter(|p| p.capabilities.network)
            .count()
            > 0) as u8)
            .to_string()
            .finish()
    })
}

fn add_plugin(body: IPCBody) -> IPCResponse {
    crate::with_state_mut!(body, state, {
        let path = PathBuf::from(body.req.to_string());
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let bytes = fs::read(&path)?;

        state.plugin_handle.load(name, bytes);

        state.mutate_config(|cfg| {
            cfg.plugins.insert(name.to_string());
        });

        ok()
    })
}

fn disable_plugin(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        let id = PluginId::new(body.req)?;

        state.plugin_handle.unload(id);

        todo!()
    })
}

fn get_all_plugins_info(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        let plugins_info = state.plugin_handle.get_all_plugins_info(|_| {});

        let payload = serde_json::to_string(&plugins_info)?;
        body.webview_sender.send_msg("plugin-info", payload);
    });

    ok()
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

        let pinfo = state.get_plugin_info(id.clone()).ok_or(IPCError::empty())?;
        let bytes = pinfo
            .get_field(&name)
            .ok_or(IPCError::empty())?
            .swap_value(&data)?
            .to_bytes()?;

        state.plugin_handle.set_config_field(id, name, bytes);

        ok()
    })
}

#[derive(Serialize)]
struct SearchResult {
    files: Vec<SampleSerialize>,
    count: usize,
}

#[derive(Debug, Deserialize)]
struct SearchRequestWithId {
    id: PluginId,
    #[serde(flatten)]
    search: SearchRequest,
}

fn plugin_search_for_sample(body: IPCBody) -> IPCResponse {
    let req: SearchRequestWithId = serde_json::from_str(&body.req)?;

    std::thread::spawn(
        move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let state = body.app_state.read().unwrap();
            let files = state.plugin_handle.search(req.id, req.search)?;
            let count = files.len();
            let res = SearchResult { files, count };

            body.webview_sender
                .send_msg("search", serde_json::to_string(&res)?);

            Ok(())
        },
    );

    ok()
}

#[derive(Deserialize)]
struct IdWithPath<'a> {
    id: PluginId,
    url: &'a str,
}

fn plugin_download_file(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        let IdWithPath { id, url } = serde_json::from_str(&body.req)?;

        let paths = state.get_config().ffpaths.clone();

        state
            .plugin_handle
            .download(id, url, paths, body.webview_sender)?
            .finish()
    })
}

fn get_plugin_paths(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        let info = state.plugin_handle.get_all_plugins_info(|icon| {
            icon.with_size(20, 20);
        });

        let lines = info
            .iter()
            .filter_map(|plug| {
                let path = app_paths::plugin_sync_path().join(plug.meta.id.as_ref());

                #[derive(Serialize)]
                struct PluginSidebarView<'a> {
                    name: &'a str,
                    path: &'a Path,
                    icon: Option<AStr>,
                }

                serde_json::to_string(&PluginSidebarView {
                    name: &plug.meta.name,
                    path: &path,
                    icon: plug.icon.clone(),
                })
                .sure("Failed to serialize PluginSidebarView")
            })
            .intersperse(",".into())
            .collect::<String>();

        format!("[{lines}]").finish()
    })
}

crate::ipc_commands! {
    IPC_PLUGINS = [
        any_online_plugin_loaded,
        add_plugin,
        disable_plugin,
        get_all_plugins_info,
        configure_plugin_value,
        plugin_search_for_sample,
        plugin_download_file,
        get_plugin_paths
    ]
}
