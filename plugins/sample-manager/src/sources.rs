//! Search plugins: WebAssembly components (`wasm32-wasip2`, the world in `wit/search.wit`) in
//! `<config dir>/sample-manager/plugins`, each run by `wasm` in a sandbox on a thread of its
//! own. A plugin's `search(query)` returns `[{ title, url, ext? }]`; a [`Source`] is the UI's side
//! of one: it sends the query, takes the
//! answer when it is ready and keeps the results the table shows. Every hit a plugin ever returned
//! is kept in a binary cache file of its own (`cache_file`), loaded at start; a search answers with
//! what the plugin found plus what the cache fuzzy-matches (so it works offline), and the Library
//! tab searches all the caches too. A remote sample is a `Sample`
//! whose `path` is its url; clicking it downloads it into the cache ([`fetch`]).
use crate::scan::Sample;
use crate::wasm::{Hit, Icon, Meta, Sandbox};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

/// A plugin's answer is cut to this many results.
const MAX_HITS: usize = 200;
const MAX_PLUGIN_BYTES: u64 = 64 << 20;
/// A sample bigger than this is not downloaded.
const MAX_DOWNLOAD: u64 = 64 << 20;

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    /// Nothing searched yet.
    Idle,
    Searching,
    Done,
    /// A search (or the plugin's loading) went wrong: what to tell the user.
    Failed(String),
}

/// Counts the work in flight (the renderer keeps drawing while it is not 0); dropping it ends it.
struct Busy(Arc<AtomicUsize>);
impl Busy {
    fn new(n: &Arc<AtomicUsize>) -> Self {
        n.fetch_add(1, SeqCst);
        Self(n.clone())
    }
}
impl Drop for Busy {
    fn drop(&mut self) {
        self.0.fetch_sub(1, SeqCst);
    }
}

enum Job {
    Search {
        id: u64,
        query: String,
        _busy: Busy,
    },
    /// The plugin's `unpack` of the bytes of a download; the answer goes to `reply`.
    Unpack {
        url: String,
        data: Vec<u8>,
        reply: Sender<Result<Vec<u8>, String>>,
    },
}

/// What a download is passed through if its hit asks for it: the plugin's `unpack`.
#[derive(Clone)]
pub struct Unpacker(Sender<Job>);

impl Unpacker {
    pub fn unpack(&self, url: &str, data: Vec<u8>) -> Result<Vec<u8>, String> {
        let stopped = || "the plugin stopped".to_string();
        let (reply, answer) = channel();
        self.0
            .send(Job::Unpack {
                url: url.into(),
                data,
                reply,
            })
            .map_err(|_| stopped())?;
        answer.recv().map_err(|_| stopped())?
    }
}

/// A search id, what it found, and the cache after it (a later snapshot has everything an earlier one
/// has, so one that is overwritten before it is taken loses nothing).
type Reply = Option<(u64, Result<Vec<Sample>, String>, Arc<Vec<Sample>>)>;

pub struct Source {
    pub name: String,
    pub description: String,
    /// The plugin's file name in the plugins folder.
    pub file: String,
    /// The network hosts the plugin may call (what it declared).
    pub hosts: Vec<String>,
    /// The icon's pixels until the UI has put them on its image sheet (`slot`).
    pub icon: Option<Icon>,
    pub slot: Option<usize>,
    pub status: Status,
    /// The last results; the `path` of each is its url.
    pub files: Arc<Vec<Sample>>,
    /// Every hit ever found (and loaded from the cache file), newest first; what the Library tab
    /// searches. Empty if the plugin did not load: nothing could unpack its downloads.
    pub cache: Arc<Vec<Sample>>,
    /// Goes up whenever `files`, `cache` or `status` change.
    pub version: u64,
    /// What was last asked.
    pub last_query: Option<String>,
    id: u64,
    busy: Arc<AtomicUsize>,
    /// `None` when the plugin did not load or its thread is gone.
    tx: Option<Sender<Job>>,
    out: Arc<Mutex<Reply>>,
}

impl Source {
    fn new(name: String, busy: Arc<AtomicUsize>) -> Self {
        Self {
            name,
            description: String::new(),
            file: String::new(),
            hosts: vec![],
            icon: None,
            slot: None,
            status: Status::Idle,
            files: Arc::default(),
            cache: Arc::default(),
            version: 0,
            last_query: None,
            id: 0,
            busy,
            tx: None,
            out: Arc::default(),
        }
    }

    fn fail(&mut self, why: String) {
        self.tx = None;
        self.cache = Arc::default();
        self.status = Status::Failed(why);
        self.version += 1;
    }

    /// A source with these results and no plugin behind it (tests).
    #[cfg(test)]
    pub fn fixed(name: &str, files: Vec<Sample>) -> Self {
        let mut s = Self::new(name.into(), Arc::default());
        (s.files, s.status) = (Arc::new(files), Status::Done);
        s
    }

    /// Asks the plugin to search `query`; an older search that is still running is dropped.
    pub fn search(&mut self, query: &str) {
        let Some(tx) = &self.tx else { return }; // did not load: keeps saying why
        self.id += 1;
        self.last_query = Some(query.to_string());
        self.status = Status::Searching;
        self.version += 1;
        let job = Job::Search {
            id: self.id,
            query: query.into(),
            _busy: Busy::new(&self.busy),
        };
        if tx.send(job).is_err() {
            self.fail("the plugin stopped".into());
        }
    }

    /// For the downloads of this plugin's hits that ask for unpacking.
    pub fn unpacker(&self) -> Option<Unpacker> {
        self.tx.clone().map(Unpacker)
    }

    /// Takes the answer of the last search if it is there.
    pub fn poll(&mut self) {
        let Some((id, answer, cache)) = self.out.lock().unwrap().take() else {
            return;
        };
        if id != self.id {
            return; // an older search finishing late
        }
        self.cache = cache;
        match answer {
            Ok(files) => (self.files, self.status) = (Arc::new(files), Status::Done),
            Err(e) => (self.files, self.status) = (Arc::default(), Status::Failed(e)),
        }
        self.version += 1;
    }

    /// What the table says while it has no rows.
    pub fn message(&self) -> String {
        match &self.status {
            Status::Idle => format!("Type a query and press Enter to search {}", self.name),
            Status::Searching => "Searching...".into(),
            Status::Done => "No results".into(),
            Status::Failed(e) => e.clone(),
        }
    }
}

// ---- reading what a plugin returns

/// `png` / `.WAV` -> `wav`; empty for anything that is not 1 to 5 letters or digits.
fn clean_ext(s: &str) -> String {
    let e = s.trim().trim_start_matches('.').to_ascii_lowercase();
    if (1..=5).contains(&e.len()) && e.bytes().all(|b| b.is_ascii_alphanumeric()) {
        e
    } else {
        String::new()
    }
}

/// The path of a url, without its query and fragment.
fn url_path(url: &str) -> &str {
    let end = url.find(['?', '#']).unwrap_or(url.len());
    let rest = url[..end].split_once("://").map_or(&url[..end], |(_, r)| r);
    rest.split_once('/').map_or("", |(_, p)| p)
}

fn url_ext(url: &str) -> Option<String> {
    let file = url_path(url).rsplit('/').next()?;
    let e = clean_ext(file.rsplit_once('.')?.1);
    (!e.is_empty()).then_some(e)
}

/// One result of `search`; `None` without an http(s) url.
fn sample(h: &Hit) -> Option<Sample> {
    let url = h.url.trim();
    let l = url.to_ascii_lowercase();
    if !(l.starts_with("http://") || l.starts_with("https://")) {
        return None;
    }
    let ext = h
        .ext
        .as_deref()
        .map(clean_ext)
        .filter(|e| !e.is_empty())
        .or_else(|| url_ext(url))
        .unwrap_or_else(|| "audio".into());
    let title = h.title.trim();
    let title = if title.is_empty() {
        let f = url_path(url).rsplit('/').next().unwrap_or("");
        f.rsplit_once('.').map_or(f, |(stem, _)| stem).to_string()
    } else {
        title.to_string()
    };
    let title = if title.is_empty() {
        "sample".to_string()
    } else {
        title
    };
    // the name carries the file type (the table and the player show it from the name)
    let name = if title.to_lowercase().ends_with(&format!(".{ext}")) {
        title
    } else {
        format!("{title}.{ext}")
    };
    Some(Sample {
        name,
        path: PathBuf::from(url),
        unpack: h.unpack,
        skip_frames: h.skip_frames as usize,
        source: None, // the worker knows whose it is
    })
}

fn samples(hits: &[Hit]) -> Vec<Sample> {
    hits.iter().filter_map(sample).take(MAX_HITS).collect()
}

// ---- loading

/// `<config dir>/sample-manager/plugins`.
pub fn plugins_dir() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join("sample-manager").join("plugins"))
}

/// Where the plugins keep their caches.
fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("sample-manager")
        .join("plugin-data")
}

/// Where the downloaded samples are kept.
fn downloads_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("sample-manager")
        .join("downloads")
}

// ---- the cache: every hit of a plugin, in a file of its own

const MAGIC: &[u8; 4] = b"SMC1";

/// A url without the part that changes between searches (a signature).
fn id_of(url: &str) -> &str {
    url.split(['?', '#']).next().unwrap_or(url)
}

/// `fresh`, then what `old` has that `fresh` has not (by `id_of` the url); nobody twice.
fn merge(fresh: &[Sample], old: &[Sample]) -> Vec<Sample> {
    let mut seen = HashSet::new();
    fresh
        .iter()
        .chain(old)
        .filter(|s| seen.insert(id_of(&s.path.to_string_lossy()).to_string()))
        .cloned()
        .collect()
}

/// Per sample: unpack (1 byte), skip_frames (u32), name and url (u32 length + bytes), all little
/// endian, after the magic.
fn encode(files: &[Sample]) -> Vec<u8> {
    let mut b = MAGIC.to_vec();
    for f in files {
        let url = f.path.to_string_lossy();
        b.push(f.unpack as u8);
        b.extend((f.skip_frames as u32).to_le_bytes());
        for s in [f.name.as_str(), &*url] {
            b.extend((s.len() as u32).to_le_bytes());
            b.extend(s.as_bytes());
        }
    }
    b
}

fn take<'a>(b: &mut &'a [u8], n: usize) -> Option<&'a [u8]> {
    let (head, tail) = b.split_at_checked(n)?;
    *b = tail;
    Some(head)
}

fn take_u32(b: &mut &[u8]) -> Option<usize> {
    Some(u32::from_le_bytes(take(b, 4)?.try_into().ok()?) as usize)
}

fn take_str(b: &mut &[u8]) -> Option<String> {
    let n = take_u32(b)?;
    Some(String::from_utf8_lossy(take(b, n)?).into_owned())
}

/// What `encode` made, every sample a hit of `source`; a wrong header gives nothing and a damaged
/// tail ends the list there (it is only a cache).
fn decode(bytes: &[u8], source: usize) -> Vec<Sample> {
    let Some(mut b) = bytes.strip_prefix(MAGIC.as_slice()) else {
        return vec![];
    };
    let mut out = vec![];
    while let Some(s) = (|| {
        Some(Sample {
            unpack: take(&mut b, 1)?[0] != 0,
            skip_frames: take_u32(&mut b)?,
            name: take_str(&mut b)?,
            path: take_str(&mut b)?.into(),
            source: Some(source),
        })
    })() {
        out.push(s);
    }
    out
}

fn read_cache(file: &Path, source: usize) -> Vec<Sample> {
    std::fs::read(file).map_or_else(|_| vec![], |b| decode(&b, source))
}

/// Writes the cache whole, and only when it is complete (a failure leaves the old file).
// the whole file per search, append-only if a cache ever grows to millions of hits
fn write_cache(file: &Path, files: &[Sample]) {
    let part = file.with_extension("part");
    let _ = file.parent().map(std::fs::create_dir_all);
    if std::fs::write(&part, encode(files))
        .and_then(|()| std::fs::rename(&part, file))
        .is_err()
    {
        let _ = std::fs::remove_file(&part);
    }
}

/// What a search shows: what the plugin found, then what the cache has for `query`.
fn answer(fresh: Vec<Sample>, cache: &[Sample], query: &str) -> Vec<Sample> {
    let local: Vec<Sample> = crate::search::search(cache, query)
        .into_iter()
        .map(|i| cache[i].clone())
        .collect();
    merge(&fresh, &local)
}

fn fnv(s: &str) -> u64 {
    s.bytes().fold(0xcbf29ce484222325, |h, b| {
        (h ^ b as u64).wrapping_mul(0x100000001b3)
    })
}

/// A file name made of what is safe in one: letters, digits, `-`, `_`, `.` and spaces.
fn safe_stem(name: &str) -> String {
    let stem = Path::new(name)
        .file_stem()
        .map_or_else(|| name.to_string(), |s| s.to_string_lossy().into_owned());
    let s: String = stem
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '-' | '_' | ' ') {
                c
            } else {
                '_'
            }
        })
        .take(80)
        .collect();
    let s = s.trim().to_string();
    if s.is_empty() { "sample".into() } else { s }
}

/// Where the downloaded sample `url` is kept: named after the sample (it is what is dragged into
/// a DAW), plus a hash of the url without its query (a signature that changes every search).
pub fn cache_path(url: &str, name: &str) -> PathBuf {
    let id = id_of(url);
    let ext = Some(clean_ext(
        &Path::new(name)
            .extension()
            .map_or(String::new(), |e| e.to_string_lossy().into_owned()),
    ))
    .filter(|e| !e.is_empty())
    .unwrap_or_else(|| "audio".into());
    downloads_dir().join(format!("{}-{:08x}.{ext}", safe_stem(name), fnv(id) as u32))
}

/// The local file of the remote sample, if it was downloaded before.
pub fn cached(url: &str, name: &str) -> Option<PathBuf> {
    Some(cache_path(url, name)).filter(|p| p.exists())
}

/// The local file of the remote sample `url` (called `name`), downloaded on first use. With an
/// `unpacker` the download goes through the plugin's `unpack` first.
pub fn fetch(url: &str, name: &str, unpacker: Option<&Unpacker>) -> Result<PathBuf, String> {
    let dest = cache_path(url, name);
    fetch_to(url, &dest, unpacker).map(|()| dest)
}

fn fetch_to(url: &str, dest: &Path, unpacker: Option<&Unpacker>) -> Result<(), String> {
    if dest.exists() {
        return Ok(());
    }
    let Some(unpacker) = unpacker else {
        return crate::wasm::download(url, dest, MAX_DOWNLOAD);
    };
    let raw = dest.with_extension("download");
    let made = crate::wasm::download(url, &raw, MAX_DOWNLOAD).and_then(|()| {
        let audio = unpacker.unpack(url, std::fs::read(&raw).map_err(|e| e.to_string())?)?;
        let part = dest.with_extension("part");
        std::fs::write(&part, audio)
            .and_then(|()| std::fs::rename(&part, dest))
            .map_err(|e| e.to_string())
    });
    let _ = std::fs::remove_file(&raw);
    made
}

fn read_plugin(path: &Path) -> Result<Vec<u8>, String> {
    let size = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
    if size > MAX_PLUGIN_BYTES {
        return Err(format!(
            "the file is {} KB, plugins may be {} KB",
            size >> 10,
            MAX_PLUGIN_BYTES >> 10
        ));
    }
    std::fs::read(path).map_err(|e| e.to_string())
}

fn worker(
    mut sandbox: Sandbox,
    rx: Receiver<Job>,
    out: Arc<Mutex<Reply>>,
    mut cache: Arc<Vec<Sample>>,
    file: PathBuf,
    index: usize,
) {
    while let Ok(first) = rx.recv() {
        let mut jobs = vec![first];
        jobs.extend(rx.try_iter());
        // every unpack is answered, of the searches only the last is worth asking
        let last = jobs.iter().rposition(|j| matches!(j, Job::Search { .. }));
        for (i, job) in jobs.into_iter().enumerate() {
            match job {
                Job::Unpack { url, data, reply } => {
                    let _ = reply.send(sandbox.unpack(&url, &data));
                }
                Job::Search { id, query, _busy } if Some(i) == last => {
                    let found = sandbox.search(&query).map(|hits| {
                        let mut fresh = samples(&hits);
                        fresh.iter_mut().for_each(|s| s.source = Some(index));
                        fresh
                    });
                    let result = match found {
                        Ok(fresh) => {
                            if !fresh.is_empty() {
                                cache = Arc::new(merge(&fresh, &cache));
                                write_cache(&file, &cache);
                            }
                            Ok(answer(fresh, &cache, &query))
                        }
                        // offline or broken: the cache still answers
                        Err(e) => match answer(vec![], &cache, &query) {
                            local if local.is_empty() => Err(e),
                            local => {
                                eprintln!("[plugin] {e}; answering from the cache");
                                Ok(local)
                            }
                        },
                    };
                    *out.lock().unwrap() = Some((id, result, cache.clone()));
                    // `_busy` ends here: the work counter goes down after the answer is there
                }
                Job::Search { .. } => {}
            }
        }
    }
}

/// Starts the plugin `path` on a thread of its own; the receiver gets what it registered once the
/// script has run.
fn start(
    index: usize,
    path: &Path,
    cache_dir: &Path,
    busy: &Arc<AtomicUsize>,
) -> (Source, Option<Receiver<Result<Meta, String>>>) {
    let stem = path
        .file_stem()
        .map_or_else(|| "plugin".into(), |s| s.to_string_lossy().into_owned());
    let label = path
        .file_name()
        .map_or_else(|| stem.clone(), |s| s.to_string_lossy().into_owned());
    let mut source = Source::new(stem.clone(), busy.clone());
    source.file = label.clone();
    let bytes = match read_plugin(path) {
        Ok(b) => b,
        Err(e) => {
            source.fail(format!("Could not load {label}: {e}"));
            return (source, None);
        }
    };
    let (tx, rx) = channel::<Job>();
    let (ready_tx, ready_rx) = channel();
    let out = source.out.clone();
    let safe: String = stem
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let file = cache_dir.join(format!("{safe}.bin"));
    let cache = Arc::new(read_cache(&file, index));
    source.cache = cache.clone();
    let spawned = std::thread::Builder::new()
        .name(format!("plugin {stem}"))
        .spawn(move || {
            // the plugin lives and dies on this thread
            let mut sandbox = Sandbox::new(&stem);
            let loaded = sandbox.load(&label, &bytes);
            let ok = loaded.is_ok();
            let _ = ready_tx.send(loaded);
            if ok {
                worker(sandbox, rx, out, cache, file, index);
            }
        });
    match spawned {
        Ok(_) => source.tx = Some(tx),
        Err(e) => {
            source.fail(format!("Could not start the plugin: {e}"));
            return (source, None);
        }
    }
    (source, Some(ready_rx))
}

/// What the plugins folder holds.
pub struct Plugins {
    /// The plugins that are loaded.
    pub sources: Vec<Source>,
    /// Every `*.wasm` of the folder, by name, and whether it is loaded (not unloaded by the user).
    pub files: Vec<(String, bool)>,
}

/// Every `*.wasm` in `dir`, by name, except the `disabled` ones (they are only listed). They start
/// side by side (compiling takes a moment) and this waits for all of them; one that fails to load
/// is a source that says why.
pub fn load_from(
    dir: &Path,
    cache_dir: &Path,
    busy: &Arc<AtomicUsize>,
    disabled: &HashSet<String>,
) -> Plugins {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("wasm"))
        })
        .filter(|p| {
            !p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with('.'))
        })
        .collect();
    paths.sort();
    let name = |p: &Path| {
        p.file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
    };
    let files = paths
        .iter()
        .map(|p| (name(p), !disabled.contains(&name(p))))
        .collect();
    paths.retain(|p| !disabled.contains(&name(p)));
    let starting: Vec<_> = paths
        .iter()
        .enumerate()
        .map(|(i, p)| start(i, p, cache_dir, busy))
        .collect();
    let sources = starting
        .into_iter()
        .map(|(mut source, ready)| {
            if let Some(ready) = ready {
                match ready.recv() {
                    Ok(Ok(meta)) => {
                        if !meta.name.trim().is_empty() {
                            source.name = meta.name.trim().to_string();
                        }
                        source.description = meta.description;
                        (source.icon, source.hosts) = (Some(meta.icon), meta.hosts);
                    }
                    Ok(Err(e)) => source.fail(format!("Could not load {}: {e}", source.name)),
                    Err(_) => source.fail(format!("{} crashed while loading", source.name)),
                }
            }
            source
        })
        .collect();
    Plugins { sources, files }
}

/// The plugins of the plugins folder (made if it is not there, so there is a place to put them).
// every change of the folder loads everything again; keep the compiled components if
// that gets slow
pub fn load_all(busy: &Arc<AtomicUsize>) -> Plugins {
    let Some(dir) = plugins_dir() else {
        return Plugins {
            sources: vec![],
            files: vec![],
        };
    };
    let _ = std::fs::create_dir_all(&dir);
    load_from(&dir, &data_dir(), busy, &crate::state::disabled_plugins())
}

/// Copies the `.wasm` file `from` into the plugins folder (and loads it from now on, if an older
/// one of that name was unloaded).
pub fn add_plugin(from: &Path) -> Result<(), String> {
    let dir = plugins_dir().ok_or("no config folder")?;
    let name = from
        .file_name()
        .filter(|_| {
            from.extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("wasm"))
        })
        .ok_or("not a .wasm file")?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let to = dir.join(name);
    // copying a file onto itself would empty it
    if std::fs::canonicalize(&to).ok() != std::fs::canonicalize(from).ok() {
        std::fs::copy(from, &to).map_err(|e| e.to_string())?;
    }
    crate::state::set_plugin_enabled(&name.to_string_lossy(), true);
    Ok(())
}

/// Deletes the plugin file `file` (a name from `Plugins::files`) from the plugins folder. Its
/// cache of hits stays: delete the `.bin` to forget them.
pub fn remove_plugin(file: &str) -> Result<(), String> {
    if file.contains(['/', '\\']) || file.starts_with('.') {
        return Err(format!("not a plugin name: {file}"));
    }
    let dir = plugins_dir().ok_or("no config folder")?;
    std::fs::remove_file(dir.join(file)).map_err(|e| e.to_string())?;
    crate::state::set_plugin_enabled(file, true); // forget the unload mark with the file
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(title: &str, url: &str, ext: Option<&str>) -> Hit {
        Hit {
            title: title.into(),
            url: url.into(),
            ext: ext.map(Into::into),
            unpack: false,
            skip_frames: 0,
        }
    }

    #[test]
    fn results_become_samples_named_with_their_file_type() {
        let r = samples(&[
            hit("Kick", "https://x/a/k.wav?sig=1", None),
            hit("Snare", "https://x/s", Some(".MP3")),
            hit("Hat.flac", "https://x/h", Some("flac")),
            hit("", "http://x/files/clap%20one.ogg#t", None),
            hit("Odd", "https://x/noext", None),
            hit("File", "file:///etc/passwd", None),
            hit("Bad ext", "https://x/y", Some("not an ext")),
        ]);
        assert!(r.iter().all(|s| !s.unpack && s.skip_frames == 0));
        let sp = Hit {
            unpack: true,
            skip_frames: 1105,
            ..hit("Kick", "https://x/k", Some("mp3"))
        };
        let sp = sample(&sp).unwrap();
        assert!(sp.unpack && sp.skip_frames == 1105);
        let got: Vec<_> = r
            .iter()
            .map(|s| (s.name.as_str(), s.path.to_str().unwrap()))
            .collect();
        assert_eq!(
            got,
            [
                ("Kick.wav", "https://x/a/k.wav?sig=1"),
                ("Snare.mp3", "https://x/s"),
                ("Hat.flac", "https://x/h"),
                ("clap%20one.ogg", "http://x/files/clap%20one.ogg#t"),
                ("Odd.audio", "https://x/noext"),
                ("Bad ext.audio", "https://x/y"),
            ]
        );
        assert_eq!(r[0].ext(), "WAV");
        assert_eq!(
            samples(&vec![hit("t", "https://x/y.wav", None); 500]).len(),
            MAX_HITS
        );
    }

    #[test]
    fn a_downloaded_sample_is_named_after_itself_and_keeps_its_name_when_the_signature_changes() {
        let a = cache_path("https://s3/x/k.mp3?sig=1", "FSS Kick/Power: 1.mp3");
        let b = cache_path("https://s3/x/k.mp3?sig=2", "FSS Kick/Power: 1.mp3");
        assert_eq!(a, b);
        assert_ne!(
            a,
            cache_path("https://s3/x/other.mp3?sig=1", "FSS Kick/Power: 1.mp3")
        );
        let name = a.file_name().unwrap().to_string_lossy().into_owned();
        assert!(
            name.starts_with("Power_ 1-") && name.ends_with(".mp3"),
            "{name}"
        );
        assert_eq!(cached("https://s3/x/never-downloaded.mp3", "n.mp3"), None);
        assert!(safe_stem("../../etc/passwd").chars().all(|c| c != '/'));
        assert_eq!(safe_stem(""), "sample");
    }

    fn stored(name: &str, url: &str) -> Sample {
        Sample {
            name: name.into(),
            path: url.into(),
            unpack: true,
            skip_frames: 1105,
            source: None,
        }
    }

    #[test]
    fn the_cache_round_trips_survives_damage_and_merges_by_url_without_the_signature() {
        let all = vec![
            stored("Kick.mp3", "https://x/k?sig=1"),
            stored("Sn\u{e4}re.mp3", "https://x/s"),
        ];
        let back = decode(&encode(&all), 3);
        assert_eq!(back.len(), 2);
        assert!(
            back.iter()
                .all(|s| s.source == Some(3) && s.unpack && s.skip_frames == 1105)
        );
        assert_eq!(
            (back[1].name.as_str(), back[1].path.to_str()),
            ("Sn\u{e4}re.mp3", Some("https://x/s"))
        );
        let bytes = encode(&all);
        assert_eq!(decode(&bytes[..bytes.len() - 3], 0).len(), 1); // cut off: the whole ones stay
        assert!(decode(b"junk", 0).is_empty() && decode(&[], 0).is_empty());

        // a hit seen again replaces the old one (new signature) and comes first, nobody twice
        let m = merge(&[stored("Kick.mp3", "https://x/k?sig=2")], &all);
        let urls: Vec<_> = m.iter().map(|s| s.path.to_str().unwrap()).collect();
        assert_eq!(urls, ["https://x/k?sig=2", "https://x/s"]);
        // offline: the cache alone answers, fuzzily
        assert_eq!(answer(vec![], &all, "knck").len(), 0);
        assert_eq!(answer(vec![], &all, "kck")[0].name, "Kick.mp3");
    }

    #[test]
    fn urls_are_cached_under_a_stable_name() {
        assert_eq!(fnv("a"), fnv("a"));
        assert_ne!(fnv("https://x/a.wav"), fnv("https://x/b.wav"));
        assert_eq!(url_ext("https://h/p/a.WAV?x=1.mp3"), Some("wav".into()));
        assert_eq!(url_ext("https://h/p/a"), None);
        assert_eq!(url_ext("https://h"), None);
    }
}
