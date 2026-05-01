use std::{collections::HashMap, io::Read, sync::mpsc};

use wasmtime::{Caller, Engine, Instance, Linker, Module, Store};

use crate::{
    AStr, AnyResult,
    plugins::{
        PluginInstance, PluginRunnerCommand as Cmd, host::HostState, manifest::PluginManifest,
    },
    state::samples::{SampleResult, SearchRequest},
};

pub(super) struct PluginRunner {
    engine: Engine,
    store: Store<HostState>,
    plugins: HashMap<AStr, PluginInstance>,
}

impl PluginRunner {
    pub fn new() -> Self {
        let engine = Engine::default();
        let store = Store::new(&engine, HostState::new());

        Self {
            engine,
            store,
            plugins: HashMap::new(),
        }
    }

    pub fn run(mut self, rx: mpsc::Receiver<Cmd>) {
        loop {
            match rx.recv() {
                Ok(Cmd::LoadPlugin { id, bytes }) => {
                    if let Err(e) = self.load_plugin(id.clone(), &bytes) {
                        eprintln!("[plugins] failed to load {id}: {e}");
                    }
                }
                Ok(Cmd::UnloadPlugin { id }) => {
                    self.plugins.remove(&id);
                }
                Ok(Cmd::Shutdown) | Err(_) => break,
            }
        }
    }

    fn load_plugin(&mut self, id: AStr, bytes: &[u8]) -> AnyResult<()> {
        let (manifest, wasm_bytes) = unpack_plugin_zip(bytes)?;

        if manifest.id != id {
            return Err(format!("manifest id '{}' does not match '{id}'", manifest.id).into());
        }

        let module = Module::new(&self.engine, &wasm_bytes)?;

        let mut linker = Linker::<HostState>::new(&self.engine);
        define_host_imports(&mut linker, &manifest)?;

        let instance = linker.instantiate(&mut self.store, &module)?;

        let fn_search = instance.get_typed_func::<(u32, u32), u32>(&mut self.store, "search")?;
        let fn_alloc = instance.get_typed_func::<u32, u32>(&mut self.store, "alloc")?;
        let fn_free = instance.get_typed_func::<(u32, u32), ()>(&mut self.store, "free")?;

        self.plugins.insert(
            id,
            PluginInstance {
                instance,
                manifest,
                fn_search,
                fn_alloc,
                fn_free,
            },
        );

        Ok(())
    }
}

fn define_host_imports(linker: &mut Linker<HostState>, manifest: &PluginManifest) -> AnyResult<()> {
    let id = manifest.id.clone();

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

            eprintln!("[plugin/{id}] {msg}");
        },
    )?;

    if manifest.capabilities.storage {
        let id = manifest.id.clone();
        linker.func_wrap(
            "host",
            "storage_get",
            move |mut caller: Caller<'_, HostState>,
                  key_ptr: u32,
                  key_len: u32,
                  out_ptr: u32,
                  out_cap: u32|
                  -> u32 {
                let mem = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .unwrap();
                let data = mem.data(&caller);
                let key: AStr = match std::str::from_utf8(
                    &data[key_ptr as usize..(key_ptr + key_len) as usize],
                ) {
                    Ok(s) => s.into(),
                    Err(_) => return u32::MAX,
                };
                match caller.data().storage.get(&(id.clone(), key)).cloned() {
                    Some(v) => {
                        let n = v.len().min(out_cap as usize);
                        mem.data_mut(&mut caller)[out_ptr as usize..out_ptr as usize + n]
                            .copy_from_slice(&v[..n]);
                        n as u32
                    }
                    None => u32::MAX,
                }
            },
        )?;

        let id = manifest.id.clone();
        linker.func_wrap(
            "host",
            "storage_set",
            move |mut caller: Caller<'_, HostState>,
                  key_ptr: u32,
                  key_len: u32,
                  val_ptr: u32,
                  val_len: u32| {
                let mem = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .unwrap();
                let data = mem.data(&caller);
                let key: AStr = match std::str::from_utf8(
                    &data[key_ptr as usize..(key_ptr + key_len) as usize],
                ) {
                    Ok(s) => s.into(),
                    Err(_) => return,
                };
                let val = data[val_ptr as usize..(val_ptr + val_len) as usize].to_vec();
                caller.data_mut().storage.insert((id.clone(), key), val);
            },
        )?;
    }

    if manifest.capabilities.encrypted_storage {
        let id = manifest.id.clone();
        linker.func_wrap(
            "host",
            "secret_get",
            move |mut caller: Caller<'_, HostState>,
                  key_ptr: u32,
                  key_len: u32,
                  out_ptr: u32,
                  out_cap: u32|
                  -> u32 {
                let mem = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .unwrap();
                let data = mem.data(&caller);
                let key: AStr = match std::str::from_utf8(
                    &data[key_ptr as usize..(key_ptr + key_len) as usize],
                ) {
                    Ok(s) => s.into(),
                    Err(_) => return u32::MAX,
                };
                match caller.data().storage.get(&(id.clone(), key)).cloned() {
                    Some(v) => {
                        let n = v.len().min(out_cap as usize);
                        mem.data_mut(&mut caller)[out_ptr as usize..out_ptr as usize + n]
                            .copy_from_slice(&v[..n]);
                        n as u32
                    }
                    None => u32::MAX,
                }
            },
        )?;

        let id = manifest.id.clone();
        linker.func_wrap(
            "host",
            "secret_set",
            move |mut caller: Caller<'_, HostState>,
                  key_ptr: u32,
                  key_len: u32,
                  val_ptr: u32,
                  val_len: u32| {
                let mem = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .unwrap();
                let data = mem.data(&caller);
                let key: AStr = match std::str::from_utf8(
                    &data[key_ptr as usize..(key_ptr + key_len) as usize],
                ) {
                    Ok(s) => s.into(),
                    Err(_) => return,
                };
                let val = data[val_ptr as usize..(val_ptr + val_len) as usize].to_vec();
                caller.data_mut().storage.insert((id.clone(), key), val);
            },
        )?;
    }

    if manifest.capabilities.network {
        let allowlist = manifest.capabilities.network_allowlist.clone();
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
                let mem = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .unwrap();
                let data = mem.data(&caller);

                let url = match std::str::from_utf8(
                    &data[url_ptr as usize..(url_ptr + url_len) as usize],
                ) {
                    Ok(s) => s.to_owned(),
                    Err(_) => return -1,
                };

                if !caller.data().is_url_allowed(&url, &allowlist) {
                    eprintln!("[plugin] blocked request to {url}");
                    return -2;
                }

                let mut headers = Vec::with_capacity(n_headers as usize);
                for i in 0..n_headers as usize {
                    let base = headers_ptr as usize + i * 16;
                    let k_ptr =
                        u32::from_le_bytes(data[base..base + 4].try_into().unwrap()) as usize;
                    let k_len =
                        u32::from_le_bytes(data[base + 4..base + 8].try_into().unwrap()) as usize;
                    let v_ptr =
                        u32::from_le_bytes(data[base + 8..base + 12].try_into().unwrap()) as usize;
                    let v_len =
                        u32::from_le_bytes(data[base + 12..base + 16].try_into().unwrap()) as usize;
                    let k = std::str::from_utf8(&data[k_ptr..k_ptr + k_len])
                        .unwrap_or("")
                        .to_owned();
                    let v = std::str::from_utf8(&data[v_ptr..v_ptr + v_len])
                        .unwrap_or("")
                        .to_owned();
                    headers.push((k, v));
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

fn read_plugin_str(
    store: &mut Store<HostState>,
    instance: &Instance,
    export_name: &str,
) -> AnyResult<String> {
    let ptr = {
        let global = instance
            .get_global(&mut *store, &format!("{export_name}_ptr"))
            .ok_or_else(|| format!("missing export: {export_name}_ptr"))?;
        global.get(&mut *store).i32().ok_or("expected i32")? as usize
    };

    let len = {
        let global = instance
            .get_global(&mut *store, &format!("{export_name}_len"))
            .ok_or_else(|| format!("missing export: {export_name}_len"))?;
        global.get(&mut *store).i32().ok_or("expected i32")? as usize
    };

    let mem = instance
        .get_memory(&mut *store, "memory")
        .ok_or("no memory export")?;
    let bytes = mem.data(&*store)[ptr..ptr + len].to_vec();
    Ok(String::from_utf8(bytes)?)
}

pub fn serialize_request(req: &SearchRequest) -> AnyResult<Vec<u8>> {
    Ok(serde_json::to_vec(req)?)
}

pub fn deserialize_results(buf: &[u8]) -> AnyResult<Vec<SampleResult>> {
    Ok(serde_json::from_slice(buf)?)
}

fn unpack_plugin_zip(bytes: &[u8]) -> AnyResult<(PluginManifest, Vec<u8>)> {
    use std::io::Read;
    let cursor = std::io::Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(cursor)?;

    let manifest: PluginManifest = {
        let mut f = zip.by_name("plugin.toml")?;
        let mut s = String::new();
        f.read_to_string(&mut s)?;
        toml::from_str(&s)?
    };

    let wasm_bytes = {
        let mut f = zip.by_name("plugin.wasm")?;
        let mut buf = vec![];
        f.read_to_end(&mut buf)?;
        buf
    };

    Ok((manifest, wasm_bytes))
}
