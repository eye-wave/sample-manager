//! The engine of the search plugins: WebAssembly components (`wasm32-wasip2`) of the world in
//! `wit/search.wit`,
//! run by wasmtime. A [`Sandbox`] is one plugin: it asks for `describe` and calls `search`.
//!
//! * A plugin can only do what the host functions allow: log and http(s) GET / POST. WASI is there (Rust's std needs it) but empty: no files, no sockets, no environment.
//! * A call that runs too long is interrupted (epochs), memory is capped. After a trap the plugin
//!   starts over (a fresh instance) on the next call.
//! * [`download`] is for the host, not for plugins.
use std::io::Read;
use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

use reqwest::blocking::{Client, Response as HttpResponse};
use reqwest::redirect::Policy;
use wasmtime::component::{Component, HasSelf, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store, StoreLimits, StoreLimitsBuilder, Trap};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

mod bindings {
    wasmtime::component::bindgen!({ path: "wit", world: "plugin" });
}
use bindings::bx::search::host::{self, Response};
use bindings::exports::bx::search::search::Info;

/// A call (one `search`) is interrupted after this long.
pub const CALL_TIMEOUT: Duration = Duration::from_secs(30);
/// A plugin's memory.
const MEMORY_MAX: usize = 256 << 20;
const HTTP_TIMEOUT: Duration = Duration::from_secs(15);
/// What `http-get` reads of a response.
const HTTP_MAX_BODY: u64 = 8 << 20;
/// What a plugin may send.
const HTTP_MAX_REQUEST: usize = 1 << 20;
const USER_AGENT: &str = "sample-manager-plugin";
/// How often the engine's clock ticks; a timeout is a number of ticks.
const TICK: Duration = Duration::from_millis(20);

// ---- http

fn check_url(url: &str) -> Result<(), String> {
    let l = url.trim_start().to_ascii_lowercase();
    if l.starts_with("http://") || l.starts_with("https://") {
        Ok(())
    } else {
        Err(format!("only http(s) urls are allowed: {url}"))
    }
}

fn agent(timeout: Duration) -> Client {
    Client::builder()
        .timeout(timeout)
        .user_agent(USER_AGENT)
        .redirect(Policy::limited(10))
        .build()
        .expect("reqwest client")
}

fn read_limited(response: &mut HttpResponse, max_bytes: u64) -> Result<Vec<u8>, String> {
    let mut body = Vec::new();

    response
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut body)
        .map_err(|e| e.to_string())?;

    if body.len() as u64 > max_bytes {
        return Err(format!("response body exceeds {max_bytes} bytes"));
    }

    Ok(body)
}

fn http_request(
    client: &Client,
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body: &[u8],
) -> Result<Response, String> {
    check_url(url)?;

    if body.len() > HTTP_MAX_REQUEST {
        return Err(format!(
            "a request body may be {} KB at most",
            HTTP_MAX_REQUEST >> 10
        ));
    }

    let method = method.to_ascii_uppercase();

    let mut req = match method.as_str() {
        "GET" if body.is_empty() => client.get(url),
        "POST" => client.post(url).body(body.to_vec()),
        "GET" => return Err("a GET has no body".into()),
        other => return Err(format!("only GET and POST are allowed, not {other}")),
    };

    for (k, v) in headers {
        req = req.header(k, v);
    }

    let mut resp = req.send().map_err(|e| e.to_string())?;

    eprintln!("[http] {method} {url}");
    for (k, v) in headers {
        eprintln!("[http] request header: {k}: {v}");
    }
    eprintln!("[http] configured user-agent: {USER_AGENT}");

    eprintln!("[http] {method} {url} -> {}", resp.status());
    for (name, value) in resp.headers() {
        eprintln!("[http]   {}: {:?}", name, value);
    }

    let status = resp.status().as_u16();
    let body = read_limited(&mut resp, HTTP_MAX_BODY)?;

    Ok(Response { status, body })
}

/// Downloads `url` (http or https) to `dest`, at most `max_bytes` of it.
/// The destination only appears when the download is complete.
/// For the host, not for plugins.
pub fn download(url: &str, dest: &Path, max_bytes: u64) -> Result<(), String> {
    check_url(url)?;

    let client = agent(Duration::from_secs(60));

    let resp = client
        .get(url)
        .send()
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;

    let part = dest.with_extension("part");

    let result = (|| -> std::io::Result<()> {
        if let Some(dir) = dest.parent() {
            std::fs::create_dir_all(dir)?;
        }

        let mut file = std::fs::File::create(&part)?;

        let copied = std::io::copy(&mut resp.take(max_bytes.saturating_add(1)), &mut file)?;

        if copied > max_bytes {
            return Err(std::io::Error::other(format!(
                "download exceeds {max_bytes} bytes"
            )));
        }

        // Ensure the complete file is written before publishing it.
        use std::io::Write;
        file.flush()?;
        drop(file);

        std::fs::rename(&part, dest)?;
        Ok(())
    })();

    result.map_err(|e| {
        let _ = std::fs::remove_file(&part);
        e.to_string()
    })
}

// ---- the host functions

/// What one plugin's instance holds: WASI (empty), the limits and the host's side of `host`.
struct State {
    wasi: WasiCtx,
    table: ResourceTable,
    limits: StoreLimits,
    name: String,
    http: Client,
}

impl WasiView for State {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl host::Host for State {
    fn log(&mut self, message: String) {
        eprintln!("[plugin {}] {message}", self.name);
    }

    fn http_get(
        &mut self,
        url: String,
        headers: Vec<(String, String)>,
    ) -> Result<Response, String> {
        http_request(&self.http, "GET", &url, &headers, &[])
    }

    fn http_request(
        &mut self,
        method: String,
        url: String,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    ) -> Result<Response, String> {
        http_request(&self.http, &method, &url, &headers, &body)
    }
}

// ---- the sandbox

/// What a plugin says about itself (`describe`).
#[derive(Debug, Clone, PartialEq)]
pub struct Meta {
    pub name: String,
    pub description: String,
}

/// One result of `search`.
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub title: String,
    pub url: String,
    pub ext: Option<String>,
    /// Whether the downloaded file goes through [`Sandbox::unpack`] first.
    pub unpack: bool,
    pub skip_frames: u32,
}

/// One engine for all plugins; a thread counts its epochs (what timeouts are measured in).
fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        let mut config = Config::new();
        config.epoch_interruption(true);
        let engine = Engine::new(&config).expect("wasmtime engine");
        let ticker = engine.clone();
        std::thread::Builder::new()
            .name("plugin clock".into())
            .spawn(move || {
                loop {
                    std::thread::sleep(TICK);
                    ticker.increment_epoch();
                }
            })
            .expect("clock thread");
        engine
    })
}

struct Instance {
    store: Store<State>,
    plugin: bindings::Plugin,
}

pub struct Sandbox {
    name: String,
    timeout: Duration,
    linker: Linker<State>,
    component: Option<Component>,
    /// `None` until the first call and again after a trap.
    instance: Option<Instance>,
}

fn message(e: &wasmtime::Error) -> String {
    if matches!(e.downcast_ref::<Trap>(), Some(Trap::Interrupt)) {
        return "the plugin took too long".into();
    }
    let all = format!("{e:#}");
    format!("the plugin crashed: {}", all.lines().next().unwrap_or(""))
}

impl Sandbox {
    /// A sandbox for the plugin `name` (it shows in its log lines).
    pub fn new(name: &str) -> Self {
        let mut linker = Linker::new(engine());
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker).expect("WASI");
        bindings::Plugin::add_to_linker::<State, HasSelf<State>>(&mut linker, |s| s)
            .expect("host functions");
        Self {
            name: name.into(),
            timeout: CALL_TIMEOUT,
            linker,
            component: None,
            instance: None,
        }
    }

    fn instantiate(&self) -> Result<Instance, String> {
        let component = self.component.as_ref().ok_or("no plugin is loaded")?;
        let state = State {
            // empty: no preopened directories, no environment, no network; stderr is the host's
            wasi: WasiCtxBuilder::new().inherit_stderr().build(),
            table: ResourceTable::new(),
            limits: StoreLimitsBuilder::new()
                .memory_size(MEMORY_MAX)
                .instances(8)
                .build(),
            name: self.name.clone(),
            http: agent(HTTP_TIMEOUT),
        };
        let mut store = Store::new(engine(), state);
        store.limiter(|s| &mut s.limits);
        store.epoch_deadline_trap();
        self.arm(&mut store);
        let plugin = bindings::Plugin::instantiate(&mut store, component, &self.linker)
            .map_err(|e| message(&e))?;
        Ok(Instance { store, plugin })
    }

    fn arm(&self, store: &mut Store<State>) {
        store.set_epoch_deadline((self.timeout.as_millis() / TICK.as_millis()).max(1) as u64);
    }

    /// Compiles the component `bytes` (`label` is its name in error messages), starts it and
    /// returns what it says about itself.
    pub fn load(&mut self, label: &str, bytes: &[u8]) -> Result<Meta, String> {
        self.instance = None;
        self.component = Some(
            Component::new(engine(), bytes)
                .map_err(|e| format!("{label} is not a plugin: {e:#}"))?,
        );
        let meta = self.with_instance(|inst| {
            inst.plugin
                .bx_search_search()
                .call_describe(&mut inst.store)
        })?;
        let Info { name, description } = meta;
        Ok(Meta { name, description })
    }

    /// Runs `f` on the instance (made if there is none, with a fresh time budget); a trap throws
    /// the instance away.
    fn with_instance<T>(
        &mut self,
        f: impl FnOnce(&mut Instance) -> wasmtime::Result<T>,
    ) -> Result<T, String> {
        let mut inst = match self.instance.take() {
            Some(i) => i,
            None => self.instantiate()?,
        };
        self.arm(&mut inst.store);
        match f(&mut inst) {
            Ok(v) => {
                self.instance = Some(inst);
                Ok(v)
            }
            Err(e) => Err(message(&e)),
        }
    }

    /// Calls the plugin's `unpack` on the bytes downloaded for one of its hits.
    pub fn unpack(&mut self, url: &str, data: &[u8]) -> Result<Vec<u8>, String> {
        self.with_instance(|inst| {
            inst.plugin
                .bx_search_search()
                .call_unpack(&mut inst.store, url, data)
        })?
    }

    /// Calls the plugin's `search`.
    pub fn search(&mut self, query: &str) -> Result<Vec<Hit>, String> {
        let hits = self.with_instance(|inst| {
            inst.plugin
                .bx_search_search()
                .call_search(&mut inst.store, query)
        })??;
        Ok(hits
            .into_iter()
            .map(|h| Hit {
                title: h.title,
                url: h.url,
                ext: h.ext,
                unpack: h.unpack,
                skip_frames: h.skip_frames,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::path::PathBuf;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("sm-wasm-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// Answers `n` requests, each with `status` and `body`.
    fn serve(
        n: usize,
        status: &'static str,
        body: &'static str,
    ) -> (String, std::thread::JoinHandle<()>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", l.local_addr().unwrap());
        let h = std::thread::spawn(move || {
            for _ in 0..n {
                let (mut c, _) = l.accept().unwrap();
                let mut buf = [0u8; 4096];
                let _ = c.read(&mut buf).unwrap();
                let r = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                c.write_all(r.as_bytes()).unwrap();
            }
        });
        (url, h)
    }

    #[test]
    fn download_is_atomic_and_size_limited() {
        let dir = tmp("dl");
        let (url, srv) = serve(1, "200 OK", "0123456789");
        let dest = dir.join("sub").join("a.wav");
        download(&url, &dest, 100).unwrap();
        assert_eq!(std::fs::read_to_string(&dest).unwrap(), "0123456789");
        srv.join().unwrap();

        let (url, srv) = serve(1, "200 OK", "0123456789");
        let big = dir.join("big.wav");
        assert!(download(&url, &big, 4).is_err());
        srv.join().unwrap();
        assert!(!big.exists() && !big.with_extension("part").exists());

        let (url, srv) = serve(1, "404 Not Found", "x");
        assert!(download(&url, &dir.join("n.wav"), 100).is_err());
        srv.join().unwrap();
        assert!(download("file:///etc/passwd", &dir.join("p.wav"), 100).is_err());
    }
}
