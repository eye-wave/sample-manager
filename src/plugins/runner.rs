use std::{
    collections::HashMap,
    io::Read,
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};

use plugin_wire::{
    WireEntry, encode_search_request, parse_frame, types::SampleType as WireSampleType,
};
use rayon::prelude::*;
use wasmtime::{Caller, Engine, Linker, Module, Store};

use crate::{
    AStr, AnyResult,
    plugins::{
        PluginId, PluginInstance, PluginRunnerCommand as Cmd,
        host::HostState,
        manifest::{PluginManifest, SearchMode},
        unpack_plugin_zip,
    },
    state::{
        app_paths,
        samples::{
            RawSampleEntry, RawSampleIndices, SampleEntrySerialize, SampleType, SearchRequest,
            filter_samples,
        },
    },
};

// -- index cache ---------------------------------------------------------------

struct IndexCache {
    entries: Vec<RawSampleEntry>,
    fetched_at: Instant,
}

// -- runner --------------------------------------------------------------------

pub(super) struct PluginRunner {
    engine: Engine,
    store: Store<HostState>,
    plugins: HashMap<PluginId, PluginInstance>,
    index_cache: HashMap<PluginId, IndexCache>,
}

impl PluginRunner {
    pub fn new() -> Self {
        let storage_path = app_paths::plugin_cache_path().join("store.db");
        let engine = Engine::default();
        let store = Store::new(&engine, HostState::new(storage_path));

        Self {
            engine,
            store,
            plugins: HashMap::new(),
            index_cache: HashMap::new(),
        }
    }

    pub fn run(mut self, rx: mpsc::Receiver<Cmd>) {
        loop {
            match rx.recv() {
                Ok(Cmd::LoadPlugin { name, bytes }) => {
                    if let Err(e) = self.load_plugin(&bytes) {
                        eprintln!("[plugins] failed to load {name}: {e}");
                    }
                }
                Ok(Cmd::UnloadPlugin { id }) => {
                    self.plugins.remove(&id);
                    self.index_cache.remove(&id);
                }
                Ok(Cmd::Search { id, req, reply_to }) => {
                    let result = self.run_search(&id, &req);
                    let _ = reply_to.send(result);
                }
                Ok(Cmd::GetAllPluginsInfo { reply_to }) => {
                    let plugin_info_list = self
                        .plugins
                        .values()
                        .map(|p| p.manifest.to_plugin_info(self.store.data()))
                        .collect();
                    let _ = reply_to.send(plugin_info_list);
                }
                Ok(Cmd::Shutdown) | Err(_) => break,
            }
        }
    }

    fn load_plugin(&mut self, bytes: &[u8]) -> AnyResult<()> {
        let (manifest, wasm_bytes) = unpack_plugin_zip(bytes)?;
        let module = Module::new(&self.engine, &wasm_bytes)?;

        let mut linker = Linker::<HostState>::new(&self.engine);
        define_host_imports(&mut linker, &manifest)?;

        let instance = linker.instantiate(&mut self.store, &module)?;

        let fn_search = instance.get_typed_func::<(u32, u32), u32>(&mut self.store, "search")?;
        let fn_alloc = instance.get_typed_func::<u32, u32>(&mut self.store, "alloc")?;
        let fn_free = instance.get_typed_func::<(u32, u32), ()>(&mut self.store, "free")?;

        let fn_get_index = instance
            .get_typed_func::<(u32, u32), u32>(&mut self.store, "get_index")
            .ok();

        let id = manifest.id.clone();

        self.plugins.insert(
            id,
            PluginInstance {
                instance,
                manifest,
                fn_search,
                fn_get_index,
                fn_alloc,
                fn_free,
            },
        );

        Ok(())
    }

    // -- search dispatch -------------------------------------------------------

    fn run_search(
        &mut self,
        id: &PluginId,
        req: &SearchRequest,
    ) -> Result<Vec<SampleEntrySerialize>, String> {
        let search_mode = self
            .plugins
            .get(id)
            .ok_or_else(|| format!("plugin {id} not loaded"))?
            .manifest
            .search_mode
            .clone();

        match search_mode {
            SearchMode::Delegated => {
                let raw = self.call_wasm_search(id, req)?;
                Ok(raw.into_iter().map(SampleEntrySerialize::from).collect())
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
                    .into_iter()
                    .map(SampleEntrySerialize::from)
                    .collect())
            }
        }
    }

    // -- wasm calls ------------------------------------------------------------

    fn call_wasm_search(
        &mut self,
        id: &PluginId,
        req: &SearchRequest,
    ) -> Result<Vec<RawSampleEntry>, String> {
        let plugin = self.plugins.get(id).ok_or("plugin not loaded")?;

        let req_bytes = encode_search_request(req.limit, req.offset, req.is_fav, &req.query);
        let (req_ptr, req_len) =
            wasm_alloc_write(&mut self.store, plugin, &req_bytes).map_err(|e| e.to_string())?;

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
            read_frame_at(&mut self.store, plugin, frame_ptr).map_err(|e| e.to_string())?;

        let plugin = self.plugins.get(id).unwrap();
        plugin
            .fn_free
            .call(&mut self.store, (frame_ptr, frame_size as u32))
            .map_err(|e| e.to_string())?;

        Ok(entries)
    }

    fn call_wasm_get_index(&mut self, id: &PluginId) -> Result<Vec<RawSampleEntry>, String> {
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
            wasm_alloc_write(&mut self.store, plugin, &config_bytes).map_err(|e| e.to_string())?;

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
            read_frame_at(&mut self.store, plugin, frame_ptr).map_err(|e| e.to_string())?;

        let plugin = self.plugins.get(id).unwrap();
        plugin
            .fn_free
            .call(&mut self.store, (frame_ptr, frame_size as u32))
            .map_err(|e| e.to_string())?;

        Ok(entries)
    }
}

// -- wasm memory helpers -------------------------------------------------------

/// Allocates `bytes` inside the plugin's wasm memory and returns (ptr, len).
fn wasm_alloc_write(
    store: &mut Store<HostState>,
    plugin: &PluginInstance,
    bytes: &[u8],
) -> AnyResult<(u32, u32)> {
    let len = bytes.len() as u32;
    let ptr = plugin.fn_alloc.call(&mut *store, len)?;
    plugin
        .instance
        .get_memory(&mut *store, "memory")
        .ok_or("plugin has no memory export")?
        .write(&mut *store, ptr as usize, bytes)?;
    Ok((ptr, len))
}

/// Copies the frame out of wasm memory and parses it via `plugin_wire::parse_frame`.
/// Returns `(entries, frame_byte_size)` — caller frees `frame_ptr` with that size.
fn read_frame_at(
    store: &mut Store<HostState>,
    plugin: &PluginInstance,
    frame_ptr: u32,
) -> AnyResult<(Vec<RawSampleEntry>, usize)> {
    let mem = plugin
        .instance
        .get_memory(&mut *store, "memory")
        .ok_or("plugin has no memory export")?;

    let base = frame_ptr as usize;
    let frame_data = {
        let raw = mem.data(&*store);
        if base + 4 > raw.len() {
            return Err("frame pointer out of bounds".into());
        }
        raw[base..].to_vec() // copy out before releasing the borrow
    };

    let (wire_entries, bytes_consumed) =
        parse_frame(&frame_data).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    let entries = wire_entries.into_iter().map(wire_entry_to_raw).collect();

    Ok((entries, bytes_consumed))
}

/// Convert a `plugin_wire::WireEntry` into the host's `RawSampleEntry`.
fn wire_entry_to_raw(e: WireEntry) -> RawSampleEntry {
    RawSampleEntry {
        str_content: e.str_content,
        indices: RawSampleIndices {
            name_end: e.name_end,
            path_end: e.path_end,
            description_end: e.description_end,
            tags_end: e.tags_end,
        },
        bpm: e.bpm,
        sample_type: match e.sample_type {
            WireSampleType::OneShot => SampleType::OneShot,
            WireSampleType::Loop => SampleType::Loop,
        },
    }
}

// -- host imports --------------------------------------------------------------

fn define_host_imports(linker: &mut Linker<HostState>, manifest: &PluginManifest) -> AnyResult<()> {
    let id = manifest.id.clone();
    let caps = manifest.capabilities.clone();

    // log(ptr, len)
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
                eprintln!("[plugin/{log_id}] {msg}");
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
                    eprintln!("[plugin/{sec_id}] denied secret_get: capability missing");
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
                    eprintln!("[plugin/{sec_id}] denied secret_set: capability missing");
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
                  out_ptr: u32,
                  out_cap: u32|
                  -> i32 {
                if !net_caps.network {
                    eprintln!("[plugin] blocked: network capability not enabled");
                    return -2;
                }
                let mem = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .unwrap();

                let (url, headers) = {
                    let data = mem.data(&caller);
                    let url = match std::str::from_utf8(
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
                    (url, headers)
                };

                if !caller.data().is_url_allowed(&url, &allowlist) {
                    eprintln!("[plugin] blocked request to {url}");
                    return -2;
                }

                let response = (|| -> AnyResult<Vec<u8>> {
                    let mut req = ureq::get(&url);
                    for (k, v) in &headers {
                        req = req.header(k, v);
                    }
                    let mut res = req.call()?;
                    let mut body = vec![];
                    res.body_mut().as_reader().read_to_end(&mut body)?;
                    Ok(body)
                })();

                match response {
                    Ok(body) => {
                        let n = body.len().min(out_cap as usize);
                        mem.data_mut(&mut caller)[out_ptr as usize..out_ptr as usize + n]
                            .copy_from_slice(&body[..n]);
                        n as i32
                    }
                    Err(e) => {
                        eprintln!("[plugin] http_fetch error: {e}");
                        -3
                    }
                }
            },
        )?;
    }

    Ok(())
}
