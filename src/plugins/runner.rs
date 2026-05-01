use std::{collections::HashMap, io::Read, sync::mpsc};
use wasmtime::{Caller, Engine, Linker, Module, Store};

use crate::{
    AStr, AnyResult,
    plugins::{
        PluginInstance, PluginRunnerCommand as Cmd, host::HostState, manifest::PluginManifest,
    },
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
                Ok(Cmd::LoadPlugin { name, bytes }) => {
                    if let Err(e) = self.load_plugin(&bytes) {
                        eprintln!("[plugins] failed to load {name}: {e}");
                    }
                }
                Ok(Cmd::UnloadPlugin { id }) => {
                    self.plugins.remove(&id);
                }
                Ok(Cmd::GetManifest { id, reply_to }) => {
                    let manifest = self.plugins.get(&id).map(|p| p.manifest.clone());
                    let _ = reply_to.send(manifest);
                }
                Ok(Cmd::ListPlugins { reply_to }) => {
                    let ids: Vec<AStr> = self.plugins.keys().cloned().collect();
                    let _ = reply_to.send(ids);
                }
                Err(err) => eprintln!("Error occured while receiving the command.\n\t{err}"),
                _ => break,
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

        let id = manifest.id.clone();

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

    let caps = manifest.capabilities.clone();

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

    let s_get_id = id.clone();
    let s_get_caps = caps.clone();
    linker.func_wrap(
        "host",
        "storage_get",
        move |mut caller: Caller<'_, HostState>,
              k_ptr: u32,
              k_len: u32,
              o_ptr: u32,
              o_cap: u32|
              -> u32 {
            if !s_get_caps.storage {
                return u32::MAX;
            }

            let mem = caller
                .get_export("memory")
                .and_then(|e| e.into_memory())
                .unwrap();
            let data = mem.data(&caller);
            let key: AStr =
                match std::str::from_utf8(&data[k_ptr as usize..(k_ptr + k_len) as usize]) {
                    Ok(s) => s.into(),
                    Err(_) => return u32::MAX,
                };

            match caller.data().storage.get(&(s_get_id.clone(), key)).cloned() {
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

    let s_set_id = id.clone();
    let s_set_caps = caps.clone();
    linker.func_wrap(
        "host",
        "storage_set",
        move |mut caller: Caller<'_, HostState>, k_ptr: u32, k_len: u32, v_ptr: u32, v_len: u32| {
            if !s_set_caps.storage {
                return;
            }

            let mem = caller
                .get_export("memory")
                .and_then(|e| e.into_memory())
                .unwrap();
            let data = mem.data(&caller);
            let key: AStr =
                match std::str::from_utf8(&data[k_ptr as usize..(k_ptr + k_len) as usize]) {
                    Ok(s) => s.into(),
                    Err(_) => return,
                };
            let val = data[v_ptr as usize..(v_ptr + v_len) as usize].to_vec();
            caller
                .data_mut()
                .storage
                .insert((s_set_id.clone(), key), val);
        },
    )?;

    let sec_get_id = id.clone();
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
                eprintln!("[plugin/{sec_get_id}] denied secret_get: capability missing");
                return u32::MAX;
            }

            let mem = caller
                .get_export("memory")
                .and_then(|e| e.into_memory())
                .unwrap();
            let data = mem.data(&caller);
            let key: AStr =
                match std::str::from_utf8(&data[k_ptr as usize..(k_ptr + k_len) as usize]) {
                    Ok(s) => s.into(),
                    Err(_) => return u32::MAX,
                };

            match caller
                .data()
                .storage
                .get(&(sec_get_id.clone(), key))
                .cloned()
            {
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

    let allowlist = manifest.capabilities.network_allowlist.clone();
    let _net_caps_enabled = manifest.capabilities.network;
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

            let data = mem.data(&caller);

            let url =
                match std::str::from_utf8(&data[url_ptr as usize..(url_ptr + url_len) as usize]) {
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

                let k_ptr = u32::from_le_bytes(data[base..base + 4].try_into().unwrap()) as usize;

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

    Ok(())
}

fn unpack_plugin_zip(bytes: &[u8]) -> AnyResult<(PluginManifest, Vec<u8>)> {
    use std::io::Read;
    let cursor = std::io::Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(cursor)?;

    let manifest: PluginManifest = {
        let mut f = zip.by_name("Manifest.toml")?;
        let mut s = String::new();
        f.read_to_string(&mut s)?;
        toml::from_str(&s)?
    };

    let _wasm_path = &manifest.assets.entry;

    let wasm_bytes = if let Some(path) = &manifest.assets.entry {
        let mut f = zip.by_name(path)?;
        let mut buf = Vec::with_capacity(f.size() as usize);
        f.read_to_end(&mut buf)?;
        buf
    } else {
        return Err("Manifest missing assets.entry wasm path".into());
    };

    Ok((manifest, wasm_bytes))
}
