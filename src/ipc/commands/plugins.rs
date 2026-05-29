use std::fs;
use std::path::{Path, PathBuf};

use plugin_wire::sample::SampleSerialize;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::ipc::{IPCBody, IPCError, IPCResponse, IntoIPCJsonResponse, IntoIPCResponse, ok};
use crate::plugins::PluginId;
use crate::state::{app_paths, samples::SearchRequest};
use crate::{AStr, LogErrorExt};

fn any_online_plugin_loaded(body: IPCBody) -> IPCResponse {
    let state = body.read_state()?;
    (state
        .plugin_handle
        .get_all_plugins_info(|_| {})
        .iter()
        .filter(|p| p.capabilities.network)
        .count()
        > 0)
    .finish()
}

fn add_plugin(body: IPCBody) -> IPCResponse {
    let mut state = body.write_state()?;

    let path = PathBuf::from(body.req.to_string());
    let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let bytes = fs::read(&path)?;

    state.plugin_handle.load(name, bytes);
    state.mutate_config(|cfg| {
        cfg.plugins.insert(name.to_string());
    });

    ok()
}

fn disable_plugin(body: IPCBody) -> IPCResponse {
    let state = body.read_state()?;
    let id = PluginId::new(&body.req)?;

    state.plugin_handle.unload(id);

    todo!()
}

fn get_all_plugins_info(body: IPCBody) -> IPCResponse {
    let state = body.read_state()?;
    let plugins_info = state.plugin_handle.get_all_plugins_info(|_| {});

    body.webview_sender.send_json("plugin-info", &plugins_info);

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
    let state = body.read_state()?;
    let ConfigPluginValueUpdate { id, name, data } = body.parse_req()?;

    let pinfo = state.get_plugin_info(id.clone()).ok_or(IPCError::empty())?;
    let bytes = pinfo
        .get_field(&name)
        .ok_or(IPCError::empty())?
        .swap_value(&data)?
        .to_bytes()?;

    state.plugin_handle.set_config_field(id, name, bytes);

    ok()
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
    let req: SearchRequestWithId = body.parse_req()?;

    std::thread::spawn(move || {
        let state = body.read_state().unwrap();
        let files = state.plugin_handle.search(req.id, req.search).unwrap();
        let count = files.len();
        let res = SearchResult { files, count };

        body.webview_sender.send_json("search", &res);
    });

    ok()
}

#[derive(Deserialize)]
struct IdWithPath<'a> {
    id: PluginId,
    url: &'a str,
}

fn plugin_download_file(body: IPCBody) -> IPCResponse {
    let state = body.read_state()?;
    let IdWithPath { id, url } = body.parse_req()?;

    let paths = state.get_config().ffpaths.clone();

    state
        .plugin_handle
        .download(id, url, paths, body.webview_sender.clone())?
        .finish_json()
}

fn get_plugin_paths(body: IPCBody) -> IPCResponse {
    let state = body.read_state()?;
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
