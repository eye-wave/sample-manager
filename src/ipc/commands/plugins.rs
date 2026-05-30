use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::ipc::{IPCBody, IPCError, IPCResponse, IntoIPCResponse, ok};
use crate::plugins::PluginId;
use crate::state::app_paths::extract_plugin_path;
use crate::state::samples::utils::sync_path;
use crate::state::samples::{SampleSerialize, SearchRequest};
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

    let plug_id = state.plugin_handle.load(name, bytes)?;
    state.mutate_config(|cfg| {
        cfg.plugins.insert(name.to_string());
    });

    let out_path = extract_plugin_path(&path);

    fs::copy(path, out_path)?;
    fs::create_dir(sync_path(&plug_id))?;

    body.webview_sender.send_ping("plug-add");

    ok()
}

fn disable_plugin(body: IPCBody) -> IPCResponse {
    let state = body.read_state()?;
    let id = PluginId::new(&body.req)?;

    state.plugin_handle.unload(id);

    todo!()
}

fn uninstall_plugin(body: IPCBody) -> IPCResponse {
    let mut state = body.write_state()?;
    let id = PluginId::new(&body.req)?;

    if let Some(name) = state.plugin_handle.uninstall(id.clone()) {
        body.webview_sender.send_msg("plug-rm", id.to_string());

        state.mutate_config(|conf| {
            conf.plugins.remove(&name);
        });

        let _ = fs::remove_dir(sync_path(&id));
    }

    ok()
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

fn plugin_search_for_sample(body: IPCBody) -> IPCResponse {
    let req: SearchRequest = body.parse_req()?;

    std::thread::spawn(move || {
        let state = body.read_state().unwrap();
        let files = state.plugin_handle.search(req).unwrap();
        let count = files.len();
        let res = SearchResult { files, count };

        body.webview_sender.send_json("search", &res);
    });

    ok()
}

#[derive(Deserialize, TS)]
#[ts(export)]
struct DownloadRequest<'a> {
    id: PluginId,
    url: &'a str,
    name: &'a Path,
}

fn plugin_download_file(body: IPCBody) -> IPCResponse {
    let state = body.read_state()?;
    let DownloadRequest { id, url, name } = body.parse_req()?;

    let paths = state.get_config().ffpaths.clone();

    let path =
        state
            .plugin_handle
            .download(id.clone(), url, name, paths, body.webview_sender.clone())?;

    if let Some(parent) = path.parent() {
        body.webview_sender
            .send_msg("plug-download", parent.to_string_lossy());
    }

    path.finish()
}

#[derive(Serialize, TS)]
#[ts(export)]
struct PluginSidebarView<'a> {
    name: &'a str,
    path: &'a Path,
    icon: Option<AStr>,
}

fn get_plugin_paths(body: IPCBody) -> IPCResponse {
    let state = body.read_state()?;
    let info = state.plugin_handle.get_all_plugins_info(|icon| {
        icon.with_size(20, 20);
    });

    let lines = info
        .iter()
        .filter_map(|plug| {
            let path = sync_path(&plug.meta.id);

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
        uninstall_plugin,
        get_all_plugins_info,
        configure_plugin_value,
        plugin_search_for_sample,
        plugin_download_file,
        get_plugin_paths
    ]
}
