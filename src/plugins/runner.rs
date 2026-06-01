use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::io::Read;
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use plugin_wire::{WireEntry, encode_search_request, parse_frame};
use rayon::prelude::*;
use wasmtime::{Caller, Engine, Linker, Module, Store};

use crate::ipc::IPCMessage;
use crate::plugins::manifest::config_key;
use crate::state::app_paths;
use crate::state::samples::{
    PluginSample, SampleEntry, SearchRequest, WaveformData, draw_audio_and_save, filter_samples,
};
use crate::{AStr, LogErrorExt};

use super::host::{HostState, PendingDownload};
use super::manifest::{ManifestError, PluginManifest, SearchMode};
use super::{PluginId, PluginInstance, PluginRunnerCommand as Cmd, PluginSendError};

#[derive(Debug, thiserror::Error)]
pub enum PluginRuntimeError {
    #[error("failed to load plugin manifest: {0}")]
    Manifest(#[from] ManifestError),

    #[error("failed to send message to plugin runtime: {0}")]
    PluginMpsc(#[from] PluginSendError),

    #[error("failed to send IPC message to webview: {0}")]
    WebviewMpsc(#[from] mpsc::SendError<IPCMessage>),

    #[error("plugin runtime I/O failure")]
    Io(#[from] std::io::Error),

    #[error("wasmtime runtime error: {0}")]
    Wasmtime(#[from] wasmtime::Error),

    #[error("HTTP request failed: {0}")]
    Ureq(#[from] ureq::Error),

    #[error("plugin '{0}' is not currently loaded")]
    PluginNotLoaded(PluginId),

    #[error("plugin '{plugin}' is missing required export '{export}'")]
    MissingExport {
        plugin: PluginId,
        export: &'static str,
    },

    #[error("plugin '{plugin}' failed while calling export '{fn_name}': {error}")]
    WasmCallError {
        plugin: PluginId,
        fn_name: &'static str,
        error: wasmtime::Error,
    },

    #[error("plugin attempted invalid memory access: {0}")]
    MemoryAccessError(#[from] wasmtime::MemoryAccessError),

    #[error("plugin '{plugin}' failed to download resource '{url}' (status code: {status})")]
    DownloadFailed {
        plugin: PluginId,
        url: String,
        status: i32,
    },

    #[error("plugin '{plugin}' attempted to read frame data outside allocated bounds")]
    FramePointerOutOfBounds { plugin: PluginId },

    #[error("plugin '{plugin}' produced an invalid frame payload: {err}")]
    FrameParseError { plugin: PluginId, err: &'static str },

    #[error(
        "plugin '{plugin}' reported a completed download for '{url}' without issuing a download request"
    )]
    MissingDownloadCall { plugin: PluginId, url: String },

    #[error("plugin '{plugin}' does not export linear memory")]
    MissingMemoryExport { plugin: PluginId },
}

// -- index cache ---------------------------------------------------------------

struct IndexCache {
    entries: Vec<WireEntry>,
    fetched_at: Instant,
}

pub type PluginSearchResult = Vec<Arc<Vec<PluginSample>>>;

// -- runner --------------------------------------------------------------------

pub(super) struct PluginRunner {
    engine: Engine,
    store: Store<HostState>,
    plugins: HashMap<PluginId, PluginInstance>,
    index_cache: HashMap<PluginId, IndexCache>,
}

impl PluginRunner {
    pub fn new() -> Result<Self, PluginRuntimeError> {
        let engine = Engine::default();
        let store = Store::new(&engine, HostState::new());

        Ok(Self {
            engine,
            store,
            plugins: HashMap::new(),
            index_cache: HashMap::new(),
        })
    }

    fn unload_plugin(&mut self, id: PluginId) {
        self.plugins.remove(&id);
        self.index_cache.remove(&id);
    }

    pub fn run(mut self, rx: mpsc::Receiver<Cmd>) {
        loop {
            match rx.recv() {
                Ok(Cmd::LoadPlugin {
                    name,
                    bytes,
                    reply_to,
                }) => match self.load_plugin(name.to_string(), &bytes) {
                    Ok(id) => {
                        let _ = reply_to.send(id);
                    }
                    Err(e) => tracing::error!(plugin = %name, error = %e, "failed to load plugin"),
                },
                Ok(Cmd::SetConfigField { id, name, data }) => {
                    self.store.data_mut().set_item(config_key(&id, &name), data);
                }
                Ok(Cmd::UnloadPlugin { id }) => self.unload_plugin(id),
                Ok(Cmd::UninstallPlugin { id, reply_to }) => {
                    let Some(plugin) = self.plugins.get(&id) else {
                        let _ = reply_to.send(None);
                        continue;
                    };

                    let filename = plugin.filename.clone();
                    let path = app_paths::plugin_path(&filename);
                    if !path.exists() {
                        let _ = reply_to.send(None);
                        continue;
                    }

                    self.unload_plugin(id);
                    if fs::remove_file(path).is_ok() {
                        let _ = reply_to.send(Some(filename));
                    }
                }
                Ok(Cmd::Search { req, reply_to }) => {
                    let plugin_ids: Vec<_> = self.plugins.keys().cloned().collect();

                    let result = plugin_ids
                        .iter()
                        .filter_map(|id| self.run_search(id, &req).sure("Failed to run search"))
                        .flatten()
                        .collect::<Vec<_>>();

                    self.store.data_mut().insert_search_cache(result.iter());

                    let reply = result
                        .iter()
                        .filter_map(|s| s.to_serialize().ok())
                        .collect::<Vec<_>>();

                    let _ = reply_to.send(Ok(reply));
                }

                Ok(Cmd::GetAllPluginsInfo { reply_to, icon_cb }) => {
                    let plugin_info_list = self
                        .plugins
                        .values()
                        .map(|p| p.manifest.to_plugin_info(self.store.data(), &icon_cb))
                        .collect();
                    let _ = reply_to.send(plugin_info_list);
                }
                Ok(Cmd::GetPluginInfo { id, reply_to }) => {
                    let reply = if let Some(plugin) = self.plugins.get(&id) {
                        Some(plugin.manifest.to_plugin_info(self.store.data(), |_| {}))
                    } else {
                        None
                    };

                    let _ = reply_to.send(reply);
                }
                Ok(Cmd::Download {
                    plugin_id,
                    url,
                    name,
                    reply_to,
                    ffpaths,
                    web_sender,
                }) => {
                    use crate::state::samples::utils::*;

                    let response = (|| -> Result<_, PluginRuntimeError> {
                        let bytes = self.call_wasm_download(&plugin_id, &url)?;

                        let mut save_path = sync_path(&plugin_id);
                        save_path.extend(name.components());

                        let ext = save_path.extension().unwrap_or(OsStr::new("wav"));

                        let _ = draw_audio_and_save(
                            Some(&plugin_id),
                            &url,
                            WaveformData::Bytes(ext, &bytes),
                            ffpaths.flatten(),
                        )
                        .map(|e| e.send_to_webview(web_sender));

                        if let Some(parent) = save_path.parent() {
                            fs::create_dir_all(parent)?;
                            fs::write(&save_path, bytes)?;
                        }

                        self.store.data_mut().insert_cached_sample(url);

                        Ok(save_path)
                    })();

                    let _ = reply_to.send(response.map_err(|e| e.to_string()));
                }
                Ok(Cmd::SearchLocalRegistry { reply_to, req }) => {
                    let samples: PluginSearchResult = self
                        .plugins
                        .keys()
                        .map(|id| self.store.data().search_local_registry_for(&req, id))
                        .collect();

                    let _ = reply_to.send(samples);
                }

                Err(_) => break,
            }
        }
    }

    fn load_plugin(
        &mut self,
        filename: String,
        bytes: &[u8],
    ) -> Result<PluginId, PluginRuntimeError> {
        let (manifest, wasm_bytes) = PluginManifest::load_from_bytes(bytes)?;
        let module = Module::new(&self.engine, &wasm_bytes)?;

        let plug_id = manifest.id.clone();

        let mut linker = Linker::<HostState>::new(&self.engine);
        define_host_imports(&mut linker, &manifest)?;

        let instance = linker.instantiate(&mut self.store, &module)?;

        let fn_search = instance.get_typed_func::<(u32, u32), u32>(&mut self.store, "search")?;
        let fn_alloc = instance.get_typed_func::<u32, u32>(&mut self.store, "alloc")?;
        let fn_free = instance.get_typed_func::<(u32, u32), ()>(&mut self.store, "free")?;

        let fn_get_index = instance
            .get_typed_func::<(u32, u32), u32>(&mut self.store, "get_index")
            .sure("fn_index not found");

        let id = manifest.id.clone();

        self.plugins.insert(
            id,
            PluginInstance {
                instance,
                filename,
                manifest,
                fn_search,
                fn_get_index,
                fn_alloc,
                fn_free,
            },
        );

        Ok(plug_id)
    }

    // -- search dispatch -------------------------------------------------------

    fn run_search(
        &mut self,
        id: &PluginId,
        req: &SearchRequest,
    ) -> Result<Vec<PluginSample>, String> {
        let search_mode = self
            .plugins
            .get(id)
            .ok_or_else(|| format!("plugin {id} not loaded"))?
            .manifest
            .search_mode
            .clone();

        match search_mode {
            SearchMode::Delegated => {
                let entries = self.call_wasm_search(id, req)?;
                Ok(entries
                    .into_iter()
                    .map(|e| PluginSample::new(e, id.clone()))
                    .collect())
            }

            SearchMode::HostIndexed { ttl_secs } => {
                let ttl = Duration::from_secs(ttl_secs);
                let needs_refresh = self
                    .index_cache
                    .get(id)
                    .map(|c| c.fetched_at.elapsed() > ttl)
                    .unwrap_or(true);

                if needs_refresh {
                    let entries = self.call_wasm_get_index(id)?;
                    self.index_cache.insert(
                        id.clone(),
                        IndexCache {
                            entries,
                            fetched_at: Instant::now(),
                        },
                    );
                }

                let cache = self.index_cache.get(id).unwrap();
                Ok(filter_samples(cache.entries.par_iter(), req)
                    .1
                    .into_iter()
                    .map(|e| PluginSample::new(e.clone(), id.clone()))
                    .collect())
            }
        }
    }

    // -- wasm calls ------------------------------------------------------------

    fn call_wasm_search(
        &mut self,
        id: &PluginId,
        req: &SearchRequest,
    ) -> Result<Vec<WireEntry>, String> {
        let plugin = self.plugins.get(id).ok_or("plugin not loaded")?;

        let req_bytes = encode_search_request(req.limit, req.offset, req.is_fav, &req.query);
        let (req_ptr, req_len) = wasm_alloc_write(id.clone(), &mut self.store, plugin, &req_bytes)
            .map_err(|e| e.to_string())?;

        let frame_ptr = plugin
            .fn_search
            .call(&mut self.store, (req_ptr, req_len))
            .map_err(|e| e.to_string())?;

        let plugin = self.plugins.get(id).unwrap();
        plugin
            .fn_free
            .call(&mut self.store, (req_ptr, req_len))
            .map_err(|e| e.to_string())?;

        let plugin = self.plugins.get(id).unwrap();
        let (entries, frame_size) =
            read_frame_at(id, &mut self.store, plugin, frame_ptr).map_err(|e| e.to_string())?;

        let plugin = self.plugins.get(id).unwrap();
        plugin
            .fn_free
            .call(&mut self.store, (frame_ptr, frame_size as u32))
            .map_err(|e| e.to_string())?;

        Ok(entries)
    }

    fn call_wasm_get_index(&mut self, id: &PluginId) -> Result<Vec<WireEntry>, String> {
        let plugin = self.plugins.get(id).ok_or("plugin not loaded")?;

        let fn_get_index = plugin
            .fn_get_index
            .clone()
            .ok_or("plugin does not export get_index")?;

        let config = self
            .store
            .data()
            .get_plugin_config(id, &plugin.manifest.config_schema);

        let config_bytes = serde_json::to_vec(&config).map_err(|e| e.to_string())?;
        let (cfg_ptr, cfg_len) =
            wasm_alloc_write(id.clone(), &mut self.store, plugin, &config_bytes)
                .map_err(|e| e.to_string())?;

        let frame_ptr = fn_get_index
            .call(&mut self.store, (cfg_ptr, cfg_len))
            .map_err(|e| e.to_string())?;

        let plugin = self.plugins.get(id).unwrap();
        plugin
            .fn_free
            .call(&mut self.store, (cfg_ptr, cfg_len))
            .map_err(|e| e.to_string())?;

        let plugin = self.plugins.get(id).unwrap();
        let (entries, frame_size) =
            read_frame_at(id, &mut self.store, plugin, frame_ptr).map_err(|e| e.to_string())?;

        let plugin = self.plugins.get(id).unwrap();
        plugin
            .fn_free
            .call(&mut self.store, (frame_ptr, frame_size as u32))
            .map_err(|e| e.to_string())?;

        Ok(entries)
    }

    fn call_wasm_download(
        &mut self,
        id: &PluginId,
        url: &str,
    ) -> Result<Vec<u8>, PluginRuntimeError> {
        let plugin = self
            .plugins
            .get(id)
            .ok_or(PluginRuntimeError::PluginNotLoaded(id.clone()))?;

        let fn_download = plugin
            .instance
            .get_typed_func::<(u32, u32), i32>(&mut self.store, "download")
            .map_err(|_| PluginRuntimeError::MissingExport {
                plugin: id.clone(),
                export: "download",
            })?;

        let (url_ptr, url_len) =
            wasm_alloc_write(id.clone(), &mut self.store, plugin, url.as_bytes())?;

        let status = fn_download
            .call(&mut self.store, (url_ptr, url_len))
            .map_err(|error| PluginRuntimeError::WasmCallError {
                plugin: id.clone(),
                fn_name: "download",
                error,
            })?;

        let plugin = self.plugins.get(id).unwrap();
        plugin
            .fn_free
            .call(&mut self.store, (url_ptr, url_len))
            .map_err(|error| PluginRuntimeError::WasmCallError {
                plugin: id.clone(),
                fn_name: "free",
                error,
            })?;

        if status != 0 {
            return Err(PluginRuntimeError::DownloadFailed {
                plugin: id.clone(),
                url: url.to_string(),
                status,
            });
        }

        self.store
            .data_mut()
            .pending_download
            .take()
            .map(|p| p.bytes)
            .ok_or_else(|| PluginRuntimeError::MissingDownloadCall {
                plugin: id.clone(),
                url: url.to_string(),
            })
    }
}

// -- wasm memory helpers -------------------------------------------------------

/// Allocates `bytes` inside the plugin's wasm memory and returns (ptr, len).
fn wasm_alloc_write(
    id: PluginId,
    store: &mut Store<HostState>,
    plugin: &PluginInstance,
    bytes: &[u8],
) -> Result<(u32, u32), PluginRuntimeError> {
    let len = bytes.len() as u32;
    let ptr = plugin.fn_alloc.call(&mut *store, len)?;
    plugin
        .instance
        .get_memory(&mut *store, "memory")
        .ok_or(PluginRuntimeError::MissingMemoryExport { plugin: id.clone() })?
        .write(&mut *store, ptr as usize, bytes)?;
    Ok((ptr, len))
}

/// Copies the frame out of wasm memory and parses it via `plugin_wire::parse_frame`.
/// Returns `(entries, frame_byte_size)` - caller frees `frame_ptr` with that size.
fn read_frame_at(
    id: &PluginId,
    store: &mut Store<HostState>,
    plugin: &PluginInstance,
    frame_ptr: u32,
) -> Result<(Vec<WireEntry>, usize), PluginRuntimeError> {
    let mem = plugin
        .instance
        .get_memory(&mut *store, "memory")
        .ok_or(PluginRuntimeError::MissingMemoryExport { plugin: id.clone() })?;

    let base = frame_ptr as usize;
    let frame_data = {
        let raw = mem.data(&*store);
        if base + 4 > raw.len() {
            return Err(PluginRuntimeError::FramePointerOutOfBounds { plugin: id.clone() });
        }
        raw[base..].to_vec() // copy out before releasing the borrow
    };

    let (wire_entries, bytes_consumed) =
        parse_frame(&frame_data).map_err(|err| PluginRuntimeError::FrameParseError {
            plugin: id.clone(),
            err,
        })?;

    Ok((wire_entries, bytes_consumed))
}

// -- host imports --------------------------------------------------------------

fn define_host_imports(
    linker: &mut Linker<HostState>,
    manifest: &PluginManifest,
) -> Result<(), PluginRuntimeError> {
    let id = manifest.id.clone();
    let caps = manifest.capabilities.clone();

    {
        let log_id = id.clone();
        linker.func_wrap(
            "host",
            "log",
            move |mut caller: Caller<'_, HostState>, ptr: u32, len: u32| {
                let mem = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .unwrap();
                let data = mem.data(&caller);
                let msg = std::str::from_utf8(&data[ptr as usize..(ptr + len) as usize])
                    .unwrap_or("<invalid utf8>");
                tracing::info!(plugin = %log_id, "{msg}");
            },
        )?;
    }

    // storage_get(k_ptr, k_len, o_ptr, o_cap) -> u32  (u32::MAX = missing/denied)
    {
        let s_id = id.clone();
        let s_caps = caps.clone();
        linker.func_wrap(
            "host",
            "storage_get",
            move |mut caller: Caller<'_, HostState>,
                  k_ptr: u32,
                  k_len: u32,
                  o_ptr: u32,
                  o_cap: u32|
                  -> u32 {
                if !s_caps.storage {
                    return u32::MAX;
                }
                let mem = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .unwrap();
                let key: AStr = {
                    let data = mem.data(&caller);
                    match std::str::from_utf8(&data[k_ptr as usize..(k_ptr + k_len) as usize]) {
                        Ok(s) => s.into(),
                        Err(_) => return u32::MAX,
                    }
                };
                match caller.data().get_item((s_id.clone(), key)) {
                    Some(v) => {
                        let n = v.len().min(o_cap as usize);
                        mem.data_mut(&mut caller)[o_ptr as usize..o_ptr as usize + n]
                            .copy_from_slice(&v[..n]);
                        n as u32
                    }
                    None => u32::MAX,
                }
            },
        )?;
    }

    // storage_set(k_ptr, k_len, v_ptr, v_len)
    {
        let s_id = id.clone();
        let s_caps = caps.clone();
        linker.func_wrap(
            "host",
            "storage_set",
            move |mut caller: Caller<'_, HostState>,
                  k_ptr: u32,
                  k_len: u32,
                  v_ptr: u32,
                  v_len: u32| {
                if !s_caps.storage {
                    return;
                }
                let mem = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .unwrap();
                let (key, val) = {
                    let data = mem.data(&caller);
                    let key: AStr = match std::str::from_utf8(
                        &data[k_ptr as usize..(k_ptr + k_len) as usize],
                    ) {
                        Ok(s) => s.into(),
                        Err(_) => return,
                    };
                    let val = data[v_ptr as usize..(v_ptr + v_len) as usize].to_vec();
                    (key, val)
                };
                caller.data_mut().set_item((s_id.clone(), key), val);
            },
        )?;
    }

    // secret_get(k_ptr, k_len, o_ptr, o_cap) -> u32
    {
        let sec_id = id.clone();
        let sec_caps = caps.clone();
        linker.func_wrap(
            "host",
            "secret_get",
            move |mut caller: Caller<'_, HostState>,
                  k_ptr: u32,
                  k_len: u32,
                  o_ptr: u32,
                  o_cap: u32|
                  -> u32 {
                if !sec_caps.encrypted_storage {
                    tracing::warn!(
                        plugin = %sec_id,
                        capability = "secret_set",
                        "capability missing"
                    );
                    return u32::MAX;
                }
                let mem = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .unwrap();
                let key: AStr = {
                    let data = mem.data(&caller);
                    match std::str::from_utf8(&data[k_ptr as usize..(k_ptr + k_len) as usize]) {
                        Ok(s) => s.into(),
                        Err(_) => return u32::MAX,
                    }
                };
                match caller.data().get_secret_item((sec_id.clone(), key)) {
                    Some(v) => {
                        let n = v.len().min(o_cap as usize);
                        mem.data_mut(&mut caller)[o_ptr as usize..o_ptr as usize + n]
                            .copy_from_slice(&v[..n]);
                        n as u32
                    }
                    None => u32::MAX,
                }
            },
        )?;
    }

    // secret_set(k_ptr, k_len, v_ptr, v_len)
    {
        let sec_id = id.clone();
        let sec_caps = caps.clone();
        linker.func_wrap(
            "host",
            "secret_set",
            move |mut caller: Caller<'_, HostState>,
                  k_ptr: u32,
                  k_len: u32,
                  v_ptr: u32,
                  v_len: u32| {
                if !sec_caps.encrypted_storage {
                    tracing::warn!(
                        plugin = %sec_id,
                        capability = "secret_set",
                        "capability missing"
                    );
                    return;
                }
                let mem = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .unwrap();
                let (key, val) = {
                    let data = mem.data(&caller);
                    let key: AStr = match std::str::from_utf8(
                        &data[k_ptr as usize..(k_ptr + k_len) as usize],
                    ) {
                        Ok(s) => s.into(),
                        Err(_) => return,
                    };
                    let val = data[v_ptr as usize..(v_ptr + v_len) as usize].to_vec();
                    (key, val)
                };
                caller
                    .data_mut()
                    .set_secret_item((sec_id.clone(), key), val);
            },
        )?;
    }

    // fs_read(path_ptr, path_len, out_ptr, out_cap) -> i32
    //   >= 0  : bytes written into out_ptr
    //     -1  : filesystem capability not granted
    //     -2  : path is not valid utf-8
    //     -3  : file not found or read error
    //
    // The host does not restrict which absolute paths are readable beyond
    // checking for traversal sequences - the user configured the path
    // explicitly in the plugin's config UI, so they own the choice.
    {
        let fs_caps = caps.clone();
        let fs_id = id.clone();
        linker.func_wrap(
            "host",
            "fs_read",
            move |mut caller: Caller<'_, HostState>,
                  path_ptr: u32,
                  path_len: u32,
                  out_ptr: u32,
                  out_cap: u32|
                  -> i32 {
                if !fs_caps.filesystem {
                    tracing::warn!(
                        plugin = %fs_id,
                        capability = "fs_read",
                        "capability missing"
                    );
                    return -1;
                }

                let mem = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .unwrap();

                let path = {
                    let data = mem.data(&caller);
                    match std::str::from_utf8(
                        &data[path_ptr as usize..(path_ptr + path_len) as usize],
                    ) {
                        Ok(s) => s.to_owned(),
                        Err(_) => return -2,
                    }
                };

                let contents = match std::fs::read(&path) {
                    Ok(b) => b,
                    Err(e) => {
                        tracing::error!(
                            plugin = %fs_id,
                            path = ?path,
                            error = %e,
                            "fs_read failed"
                        );
                        return -3;
                    }
                };

                let n = contents.len().min(out_cap as usize);
                mem.data_mut(&mut caller)[out_ptr as usize..out_ptr as usize + n]
                    .copy_from_slice(&contents[..n]);

                n as i32
            },
        )?;
    }

    // http_fetch(url_ptr, url_len, headers_ptr, n_headers, out_ptr, out_cap) -> i32
    //   >= 0  : bytes written
    //     -1  : invalid utf-8 in url
    //     -2  : blocked (no network cap or url not in allowlist)
    //     -3  : request error
    {
        let allowlist = manifest.capabilities.network_allowlist.clone();
        let net_caps = caps.clone();
        linker.func_wrap(
            "host",
            "http_fetch",
            move |mut caller: Caller<'_, HostState>,
                  url_ptr: u32,
                  url_len: u32,
                  headers_ptr: u32,
                  n_headers: u32,
                  body_ptr: u32,
                  body_len: u32,
                  out_ptr: u32,
                  out_cap: u32|
                  -> i32 {
                if !net_caps.network {
                    tracing::warn!("network capability not enabled");
                    return -2;
                }
                let mem = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .unwrap();

                let (uri, body, headers) = {
                    let data = mem.data(&caller);
                    let uri = match std::str::from_utf8(
                        &data[url_ptr as usize..(url_ptr + url_len) as usize],
                    ) {
                        Ok(s) => s.to_owned(),
                        Err(_) => return -1,
                    };
                    let mut headers = Vec::with_capacity(n_headers as usize);
                    for i in 0..n_headers as usize {
                        let base = headers_ptr as usize + i * 16;
                        let k_ptr =
                            u32::from_le_bytes(data[base..base + 4].try_into().unwrap()) as usize;
                        let k_len = u32::from_le_bytes(data[base + 4..base + 8].try_into().unwrap())
                            as usize;
                        let v_ptr =
                            u32::from_le_bytes(data[base + 8..base + 12].try_into().unwrap())
                                as usize;
                        let v_len =
                            u32::from_le_bytes(data[base + 12..base + 16].try_into().unwrap())
                                as usize;
                        let k = std::str::from_utf8(&data[k_ptr..k_ptr + k_len])
                            .unwrap_or("")
                            .to_owned();
                        let v = std::str::from_utf8(&data[v_ptr..v_ptr + v_len])
                            .unwrap_or("")
                            .to_owned();
                        headers.push((k, v));
                    }
                    let body = if body_len > 0 {
                        Some(&data[body_ptr as usize..(body_ptr + body_len) as usize])
                    } else {
                        None
                    };
                    (uri, body, headers)
                };

                if !caller.data().is_url_allowed(&uri, &allowlist) {
                    tracing::warn!(uri = %uri, "blocked outbound request");
                    return -2;
                }

                let response = (|| -> Result<Vec<u8>, PluginRuntimeError> {
                    let mut res_body = vec![];

                    if let Some(body) = body {
                        let mut req = ureq::post(uri);
                        for (k, v) in &headers {
                            req = req.header(k, v);
                        }
                        let mut res = req.send(body)?;

                        res.body_mut().as_reader().read_to_end(&mut res_body)?;
                    } else {
                        let mut req = ureq::get(uri);
                        for (k, v) in &headers {
                            req = req.header(k, v);
                        }
                        let mut res = req.call()?;

                        res.body_mut().as_reader().read_to_end(&mut res_body)?;
                    };

                    Ok(res_body)
                })();

                match response {
                    Ok(body) => {
                        let n = body.len().min(out_cap as usize);
                        mem.data_mut(&mut caller)[out_ptr as usize..out_ptr as usize + n]
                            .copy_from_slice(&body[..n]);
                        n as i32
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "http_fetch failed");
                        -3
                    }
                }
            },
        )?;
    }

    linker.func_wrap(
        "host",
        "emit_download",
        move |mut caller: Caller<'_, HostState>, bytes_ptr: u32, bytes_len: u32| -> i32 {
            let mem = caller
                .get_export("memory")
                .and_then(|e| e.into_memory())
                .unwrap();

            let data = mem.data(&caller);
            let bytes = data[bytes_ptr as usize..(bytes_ptr + bytes_len) as usize].to_vec();

            caller.data_mut().pending_download = Some(PendingDownload { bytes });
            0
        },
    )?;

    Ok(())
}
