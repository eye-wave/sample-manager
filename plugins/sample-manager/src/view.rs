//! The screen, as a `bx_ui` tree (sample data for now). `view` knows the window size only for the
//! number of waveform bars; everything else is the layout engine's job.
use crate::audio::{Clip, Player, decode};
use crate::icons::{self, GRIP};
use crate::scan::{Dir, Sample, Tree};
use crate::search::search;
use crate::sources::{self, Source};
use crate::state::Settings;
use crate::theme::{t, use_scheme};
use bx_app::bx_ui::*;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::sync::{Arc, Mutex};

/// `Layout::rects` id of the search box.
pub const SEARCH: u32 = 1;
/// `Layout::rects` id of the folder `Dir::id` is `id - FOLDER0` (below `TRACK0`).
pub const FOLDER0: u32 = 1000;
/// `Layout::rects` id of the tree's `k`-th row (counting all rows, shown or not) is `TRACK0 + k`,
/// for the rows that are files.
pub const TRACK0: u32 = 1 << 20;
/// `Layout::rects` ids of the tree's scrollbar, the pause / play button and the waveform.
pub const TREE_SCROLL: u32 = 5;
pub const PLAY: u32 = 6;
pub const WAVE: u32 = 7;
/// `Layout::rects` id of the "Playback mode" label in the status bar.
pub const PLAYMODE: u32 = 8;
/// `Layout::rects` ids of the settings button (it is above `TRACK0`, so `Ui::event` looks for it
/// first) and, in `settings.rs`, of what the dialog holds.
pub const SETTINGS_BTN: u32 = 1 << 24;
/// `Layout::rects` ids of the heart of the table row `k` (`HEART0 + k`), of the player's heart and
/// of the two tabs; all above `TRACK0`, `Ui::event` looks for them before the rows.
pub const HEART0: u32 = 1 << 25;
pub const PLAYER_HEART: u32 = 1 << 26;
pub const NAV_LIBRARY: u32 = PLAYER_HEART + 1;
pub const NAV_FAVORITES: u32 = PLAYER_HEART + 2;
/// The nav row of the search plugin `i` is `NAV_PLUGIN0 + i`.
pub const NAV_PLUGIN0: u32 = PLAYER_HEART + 100;
/// `Layout::rects` id of the "Add Folder" button.
pub const ADD: u32 = 9;
/// `Layout::rects` id of the file tree's area and of the table rows' area (`TREE_RESERVED` and
/// `TABLE_RESERVED` are checked against them in the tests).
pub const TREE: u32 = 2;
pub const ROWS: u32 = 3;
/// `Layout::rects` id of the table's scrollbar (the whole track).
pub const SCROLL: u32 = 4;
/// `Layout::rects` id of the table row `k` (the k-th shown) is `ROW0 + k`; below `FOLDER0`.
pub const ROW0: u32 = 10;
/// Height of everything in the window except the file tree.
const TREE_RESERVED: f32 = 405.0;
/// Same for the table rows: the table scrolls by whole rows, there is no clipping.
const TABLE_RESERVED: f32 = 282.0;
/// A table row and its separator.
const ROW_H: f32 = 37.0;
const THUMB_MIN: f32 = 24.0;
/// Waveform columns drawn at most (a window's width in px, up to this): a column is a quad, 12 of
/// the 65536 vertices a scene may have.
const COLS_MAX: usize = 1600;

/// How many table rows fit in a window `h` px tall.
pub fn visible_rows(h: u32) -> usize {
    ((h as f32 - TABLE_RESERVED) / ROW_H).max(1.0) as usize
}
/// The scrollbar thumb: the share of the track that is visible, at least `THUMB_MIN`.
fn thumb_h(vis: usize, total: usize) -> f32 {
    (vis as f32 * ROW_H * vis as f32 / total as f32).max(THUMB_MIN)
}

/// A row of the file tree: a folder (its depth, open or not) or the `k`-th file of one.
enum Row<'a> {
    Folder(&'a Dir, f32, bool),
    Track(&'a Dir, usize, f32),
}
impl Row<'_> {
    fn height(&self) -> f32 {
        match self {
            Row::Folder(..) => 21.0,
            Row::Track(..) => 17.0,
        }
    }
}
/// Every row of the tree, top to bottom: roots start open, everything else closed.
fn tree_rows<'a>(roots: &'a [Dir], toggled: &HashSet<u32>) -> Vec<Row<'a>> {
    fn go<'a>(d: &'a Dir, depth: f32, t: &HashSet<u32>, out: &mut Vec<Row<'a>>) {
        let open = (depth == 0.0) != t.contains(&d.id);
        out.push(Row::Folder(d, depth, open));
        if open {
            d.dirs.iter().for_each(|c| go(c, depth + 1.0, t, out));
            out.extend((0..d.files.len()).map(|k| Row::Track(d, k, depth)));
        }
    }
    let mut out = vec![];
    roots.iter().for_each(|r| go(r, 0.0, toggled, &mut out));
    out
}
/// The height the tree has in a window `h` px tall.
/// `n_sources` search plugins add a nav row (34 px and 2 px spacing) each above it.
fn tree_avail(h: u32, n_sources: usize) -> f32 {
    (h as f32 - TREE_RESERVED - 36.0 * n_sources as f32).max(0.0)
}
/// How many rows from `first` on fit in `avail` px.
fn fit(rows: &[Row], first: usize, avail: f32) -> usize {
    let mut used = 0.0;
    rows.iter()
        .skip(first)
        .take_while(|r| {
            used += r.height();
            used <= avail
        })
        .count()
}
/// The last row the tree can start at, and the thumb's height; `None` if everything fits.
fn tree_bar(rows: &[Row], avail: f32) -> Option<(usize, f32)> {
    let last = rows.len() - fit_from_end(rows, avail);
    let total: f32 = rows.iter().map(Row::height).sum();
    (last > 0).then(|| {
        (
            last,
            (avail * avail / total).clamp(THUMB_MIN.min(avail), avail),
        )
    })
}
fn fit_from_end(rows: &[Row], avail: f32) -> usize {
    let mut used = 0.0;
    rows.iter()
        .rev()
        .take_while(|r| {
            used += r.height();
            used <= avail
        })
        .count()
}

/// The sample in the player.
pub struct Loaded {
    pub path: PathBuf,
    pub name: String,
    pub ext: String,
    /// `None`: still decoding (`loading`) or it could not be decoded.
    pub clip: Option<Arc<Clip>>,
    pub loading: bool,
    /// Why it could not be played, if it could not.
    pub error: Option<String>,
    /// A search plugin's result: `path` is its url, there is no heart and nothing to drag out.
    pub remote: bool,
}

/// What the table lists: everything scanned, or only the favorites.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Tab {
    #[default]
    Library,
    Favorites,
    /// The results of the search plugin `i`.
    Plugin(usize),
}

/// Work for the worker thread: both block (a dialog, a scan).
pub enum Job {
    AddFolder,
    Remove(PathBuf),
}

/// What a decode thread hands over: the load it was for, and the clip (`None`: it failed).
type Decoded = Option<(u64, Result<Arc<Clip>, String>)>;

/// What the screen remembers between frames.
pub struct Ui {
    pub search: TextBox,
    /// Where the named nodes were put last frame, to tell what a click hit.
    pub rects: Rects,
    /// The tracked folders, filled in by the scan thread.
    pub tree: Tree,
    /// Folders (`Dir::id`) flipped from their default: roots start open, everything else closed.
    pub toggled: HashSet<u32>,
    /// What the search box searches (`results` are indices into it): the Library and Favorites
    /// tabs have the scanned files (`lib`) and then every plugin's cache; a plugin tab has its
    /// results.
    pub files: Arc<Vec<Sample>>,
    /// The scanned files `files` was last made from.
    pub lib: Arc<Vec<Sample>>,
    pub query: String,
    pub results: Vec<usize>,
    /// The first table row shown (an index into `results`).
    pub first: usize,
    /// A scrollbar drag: where on the thumb (px from its top) it was grabbed.
    pub drag: Option<f32>,
    /// The same for the file tree: its first shown row (of all rows) and its scrollbar drag.
    pub tree_first: usize,
    pub tree_drag: Option<f32>,
    /// The player: empty until a table row is clicked.
    pub current: Option<Loaded>,
    /// Where the sample is played.
    pub player: Arc<Player>,
    /// The file under the mouse that a press picks up to drag out of the window (set each frame
    /// by `scene`, handed to the window by `render::run`).
    pub hover_src: Option<std::path::PathBuf>,
    /// Decoding runs on a thread of its own (a long file would freeze the window): the number of
    /// those running, what the last finished one made, and which load is the current one.
    pub busy: Arc<AtomicUsize>,
    /// What the click handlers ask of the worker thread (`render::run`): it picks folders and
    /// rescans, which block.
    pub jobs: Arc<Mutex<Vec<Job>>>,
    /// The settings and whether their dialog is open; `settings_to_save` is taken by the worker.
    pub settings: Settings,
    pub settings_open: bool,
    pub settings_to_save: Option<Settings>,
    /// The favorite files, the tab shown, and what `results` was last computed for (the tab and,
    /// in the favorites tab, `fav_version`); `fav_to_save` is taken by the worker.
    pub favorites: HashSet<PathBuf>,
    pub tab: Tab,
    fav_version: u64,
    results_key: (Tab, u64),
    pub fav_to_save: Option<HashSet<PathBuf>>,
    /// The first folder shown in the dialog's list.
    pub dlg_first: usize,
    /// The search plugins (the sidebar lists them) and where the thread that loads them leaves them.
    pub sources: Vec<Source>,
    pub sources_slot: Arc<Mutex<Option<Vec<Source>>>>,
    incoming: Arc<Mutex<Decoded>>,
    load_id: u64,
}
impl Ui {
    pub fn new(tree: Tree, player: Arc<Player>) -> Self {
        Self {
            search: TextBox::new(""),
            rects: Rects::default(),
            tree,
            toggled: HashSet::new(),
            files: Arc::default(),
            lib: Arc::default(),
            query: String::new(),
            results: vec![],
            first: 0,
            drag: None,
            tree_first: 0,
            tree_drag: None,
            current: None,
            player,
            hover_src: None,
            busy: Arc::default(),
            jobs: Arc::default(),
            settings: Settings::default(),
            settings_open: false,
            settings_to_save: None,
            dlg_first: 0,
            favorites: HashSet::new(),
            tab: Tab::Library,
            fav_version: 0,
            results_key: (Tab::Library, 0),
            fav_to_save: None,
            sources: vec![],
            sources_slot: Arc::default(),
            incoming: Arc::default(),
            load_id: 0,
        }
    }

    /// A click on the `k`-th shown table row: that sample replaces the one in the player and the
    /// plugin plays it (one that does not decode silences the old one).
    pub fn select(&mut self, k: usize) {
        let Some(f) = self
            .results
            .get(self.first + k)
            .and_then(|&i| self.files.get(i))
            .cloned()
        else {
            return;
        };
        // a hit of a plugin (in its tab or found in the Library) is downloaded, and unpacked by it
        let unpacker = f
            .source
            .filter(|_| f.unpack)
            .and_then(|i| self.sources.get(i))
            .and_then(Source::unpacker);
        self.load(&f, f.source.is_some(), unpacker);
    }

    /// A click on the tree's row `i` (see `TRACK0`): a file plays like a table row does.
    pub fn select_track(&mut self, i: usize) {
        if let Some(f) = self.track_sample(i) {
            self.load(&f, false, None);
        }
    }

    /// The file at row `i` of the tree (`None` for a folder or no such row).
    fn track_sample(&self, i: usize) -> Option<Sample> {
        let g = self.tree.lock().unwrap();
        match tree_rows(&g.as_ref()?.roots, &self.toggled).get(i) {
            Some(&Row::Track(d, k, _)) => Some(Sample {
                name: d.files[k].clone(),
                path: d.path.join(&d.files[k]),
                ..Default::default()
            }),
            _ => None,
        }
    }

    /// The file a press on the node `id` picks up to drag out of the window: a table row or a
    /// file in the tree (see `ROW0`, `TRACK0`).
    pub fn source_of(&self, id: u32) -> Option<std::path::PathBuf> {
        if (ROW0..FOLDER0).contains(&id) {
            let k = (id - ROW0) as usize;
            // a downloaded sample of a plugin can be dragged into a DAW; one not downloaded yet is
            // downloaded by the click that starts the press, the next press drags it
            self.row_path(k).or_else(|| {
                let f = self.files.get(*self.results.get(self.first + k)?)?;
                sources::cached(&f.path.to_string_lossy(), &f.name)
            })
        } else if id >= TRACK0 {
            self.track_sample((id - TRACK0) as usize).map(|f| f.path)
        } else {
            None
        }
    }

    /// Starts decoding `f`; the old sample goes quiet now and the new one plays once it is ready
    /// (`refresh` picks it up). A load that is no longer the last one is dropped when it finishes.
    fn load(&mut self, f: &Sample, remote: bool, unpacker: Option<sources::Unpacker>) {
        self.load_id += 1;
        self.player.stop();
        self.current = Some(Loaded {
            path: f.path.clone(),
            name: f.name.clone(),
            ext: f.ext(),
            clip: None,
            loading: true,
            error: None,
            remote,
        });
        let (id, path, name, skip) = (self.load_id, f.path.clone(), f.name.clone(), f.skip_frames);
        let (busy, slot) = (self.busy.clone(), self.incoming.clone());
        busy.fetch_add(1, SeqCst);
        std::thread::spawn(move || {
            // a malformed file can panic inside a decoder: that is "cannot decode", not a stuck `busy`
            let clip = std::panic::catch_unwind(|| {
                // a remote sample is downloaded first (once: it stays in the cache)
                let file = if remote {
                    sources::fetch(&path.to_string_lossy(), &name, unpacker.as_ref())?
                } else {
                    path.clone()
                };
                decode(&file)
                    .map(|c| Arc::new(if skip > 0 { c.skip(skip) } else { c }))
                    .map_err(|e| {
                        if remote {
                            let _ = std::fs::remove_file(&file); // do not keep what is not audio
                        }
                        format!("Cannot decode this file ({e})")
                    })
            })
            .unwrap_or_else(|_| Err("Cannot decode this file".into()));
            // a slower, older load must not replace the newer result that is waiting
            let mut s = slot.lock().unwrap();
            if s.as_ref().is_none_or(|(i, _)| *i < id) {
                *s = Some((id, clip));
            }
            drop(s);
            busy.fetch_sub(1, SeqCst);
        });
    }

    /// Takes what a decode thread finished, if it is still the sample wanted, and plays it.
    fn take_decoded(&mut self) {
        let Some((id, clip)) = self.incoming.lock().unwrap().take() else {
            return;
        };
        if let (true, Some(l)) = (id == self.load_id, self.current.as_mut()) {
            match clip {
                Ok(c) => {
                    self.player.play(c.clone());
                    (l.clip, l.error) = (Some(c), None);
                }
                Err(e) => {
                    self.player.play(Default::default());
                    (l.clip, l.error) = (None, Some(e));
                }
            }
            l.loading = false;
        }
    }

    /// Waits for the decode in flight and applies it (tests).
    #[cfg(test)]
    pub(crate) fn settle(&mut self) {
        while self.busy.load(SeqCst) > 0 {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        self.take_decoded();
    }

    /// A click at `x` on the waveform: the sample plays on from there.
    pub fn seek(&self, x: f32) {
        let (Some(r), Some(Loaded { clip: Some(c), .. })) = (self.rect_of(WAVE), &self.current)
        else {
            return;
        };
        let f = ((x - r[0]) / r[2]).clamp(0.0, 1.0);
        self.player
            .seek(((f * c.frames.len() as f32) as usize).min(c.frames.len() - 1));
    }

    pub fn is_favorite(&self, p: &Path) -> bool {
        self.favorites.contains(p)
    }

    /// The heart: adds `p` to the favorites or takes it out.
    pub fn toggle_favorite(&mut self, p: &Path) {
        if !self.favorites.remove(p) {
            self.favorites.insert(p.to_path_buf());
        }
        self.fav_version += 1;
        self.fav_to_save = Some(self.favorites.clone());
    }

    /// The path of the file in table row `k` (the k-th shown).
    pub fn row_path(&self, k: usize) -> Option<PathBuf> {
        let f = self.files.get(*self.results.get(self.first + k)?)?;
        f.source.is_none().then(|| f.path.clone()) // urls: no heart, no drag
    }

    /// The UI scale as a factor (1.0 = 100 %).
    pub fn scale(&self) -> f32 {
        self.settings.scale as f32 / 100.0
    }

    /// Searches again if the scan has finished or the search text changed.
    pub fn refresh(&mut self) {
        self.take_decoded();
        if let Some(v) = self.sources_slot.lock().unwrap().take() {
            self.sources = v;
        }
        self.sources.iter_mut().for_each(Source::poll);
        if matches!(self.tab, Tab::Plugin(i) if i >= self.sources.len()) {
            self.tab = Tab::Library;
        }
        if let Tab::Plugin(i) = self.tab {
            // a plugin's results are what its search found: `files` is that list, all of it shown
            let s = &self.sources[i];
            let key = (self.tab, s.version);
            if key != self.results_key || !Arc::ptr_eq(&s.files, &self.files) {
                self.files = s.files.clone();
                self.results = (0..self.files.len()).collect();
                self.query = self.search.text.clone();
                self.results_key = key;
                self.first = 0;
            }
            return;
        }
        let lib = match &*self.tree.lock().unwrap() {
            Some(s) => s.files.clone(),
            None => return,
        };
        // the favorites tab depends on the favorites, both on the plugins' caches (their versions
        // only go up)
        let key = (
            self.tab,
            self.sources.iter().map(|s| s.version).sum::<u64>()
                + if self.tab == Tab::Favorites {
                    self.fav_version
                } else {
                    0
                },
        );
        let stale = !Arc::ptr_eq(&lib, &self.lib) || key != self.results_key;
        if !stale && self.search.text == self.query {
            return;
        }
        if stale {
            // the scanned files, then every cache: one list for one search (and the scanned ones
            // are its first `lib.len()`)
            self.files = if self.sources.iter().all(|s| s.cache.is_empty()) {
                lib.clone()
            } else {
                let caches = self.sources.iter().flat_map(|s| s.cache.iter());
                Arc::new(lib.iter().chain(caches).cloned().collect())
            };
            self.lib = lib;
        }
        let text = self.search.text.trim();
        self.results = match self.tab {
            // the search sees the favorites only: look in them, then map back to `files`
            Tab::Favorites => {
                let (at, favs): (Vec<usize>, Vec<Sample>) = self
                    .lib
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| self.favorites.contains(&f.path))
                    .map(|(i, f)| (i, f.clone()))
                    .unzip();
                search(&favs, text).into_iter().map(|j| at[j]).collect()
            }
            // no query lists the scanned files only, a query looks in the caches too
            _ if text.is_empty() => search(&self.lib, text),
            _ => search(&self.files, text),
        };
        self.query = self.search.text.clone();
        self.results_key = key;
        self.first = 0;
    }

    /// Switches the tab. A search plugin shows what its last search found; the text already in the
    /// box is searched at once if the plugin has not searched it yet.
    pub fn set_tab(&mut self, tab: Tab) {
        self.tab = tab;
        if let Tab::Plugin(i) = tab {
            let q = self.search.text.trim().to_string();
            if let Some(s) = self.sources.get_mut(i)
                && !q.is_empty()
                && s.last_query.as_deref() != Some(q.as_str())
            {
                s.search(&q);
            }
        }
        self.refresh();
    }

    /// Enter in the search box: a plugin tab sends the text to its plugin.
    pub fn submit_search(&mut self) {
        if let Tab::Plugin(i) = self.tab {
            let q = self.search.text.trim().to_string();
            if let Some(s) = self.sources.get_mut(i).filter(|_| !q.is_empty()) {
                s.search(&q);
            }
        }
    }

    /// Keeps the table from scrolling past its last row (also after a resize).
    pub fn clamp(&mut self, h: u32) {
        self.first = self
            .first
            .min(self.results.len().saturating_sub(visible_rows(h)));
        let g = self.tree.lock().unwrap();
        if let Some(scan) = g.as_ref() {
            let rows = tree_rows(&scan.roots, &self.toggled);
            let last = rows.len() - fit_from_end(&rows, tree_avail(h, self.sources.len()));
            self.tree_first = self.tree_first.min(last);
        }
    }

    /// The last row the tree can start at and its thumb's height, if it scrolls at all.
    fn tree_bar(&self, h: u32) -> Option<(usize, f32)> {
        let g = self.tree.lock().unwrap();
        tree_bar(
            &tree_rows(&g.as_ref()?.roots, &self.toggled),
            tree_avail(h, self.sources.len()),
        )
    }

    /// Like `grab`, for the tree's scrollbar.
    pub fn grab_tree(&mut self, y: f32, h: u32) {
        let (Some(r), Some((last, th))) = (self.rect_of(TREE_SCROLL), self.tree_bar(h)) else {
            return;
        };
        let top =
            r[1] + (tree_avail(h, self.sources.len()) - th) * self.tree_first as f32 / last as f32;
        self.tree_drag = Some(if (top..top + th).contains(&y) {
            y - top
        } else {
            th / 2.0
        });
        self.drag_tree(y, h);
    }

    /// Like `drag_to`, for the tree.
    pub fn drag_tree(&mut self, y: f32, h: u32) {
        let Some(off) = self.tree_drag else { return };
        let (Some(r), Some((last, th))) = (self.rect_of(TREE_SCROLL), self.tree_bar(h)) else {
            return;
        };
        let span = tree_avail(h, self.sources.len()) - th;
        if span > 0.0 {
            let frac = ((y - off - r[1]) / span).clamp(0.0, 1.0);
            self.tree_first = (frac * last as f32).round() as usize;
        }
    }

    /// Where the thumb starts, px from the top of the track.
    fn thumb_top(&self, h: u32) -> f32 {
        let (vis, total) = (visible_rows(h), self.results.len());
        if total <= vis {
            return 0.0;
        }
        let span = vis as f32 * ROW_H - thumb_h(vis, total);
        span * self.first as f32 / (total - vis) as f32
    }

    /// Where the node `id` was put last frame.
    fn rect_of(&self, id: u32) -> Option<Rect> {
        self.rects.rect(id)
    }

    /// A click at `y` on the scrollbar: the thumb is grabbed where it is, or the track click
    /// centres it there; either way the drag goes on until `Event::Release`.
    pub fn grab(&mut self, y: f32, h: u32) {
        let Some(ty) = self.rect_of(SCROLL).map(|r| r[1]) else {
            return;
        };
        let (vis, total) = (visible_rows(h), self.results.len());
        let (top, th) = (ty + self.thumb_top(h), thumb_h(vis, total));
        self.drag = Some(if (top..top + th).contains(&y) {
            y - top
        } else {
            th / 2.0
        });
        self.drag_to(y, h);
    }

    /// Scrolls so that the thumb follows the mouse at `y` (px from the top of the window).
    pub fn drag_to(&mut self, y: f32, h: u32) {
        let (Some(off), Some(ty)) = (self.drag, self.rect_of(SCROLL).map(|r| r[1])) else {
            return;
        };
        let (vis, total) = (visible_rows(h), self.results.len());
        if total <= vis {
            return;
        }
        let span = vis as f32 * ROW_H - thumb_h(vis, total);
        let frac = ((y - off - ty) / span).clamp(0.0, 1.0);
        self.first = (frac * (total - vis) as f32).round() as usize;
    }
}

fn line() -> Node {
    space().size(Fill(1), Fixed(1.0)).bg(t().line, 0.0)
}
fn vline() -> Node {
    space().size(Fixed(1.0), Fill(1)).bg(t().line, 0.0)
}
fn chip(s: &str) -> Node {
    text(s, 10.0, t().chip_text)
        .pad_xy(7.0, 3.0)
        .bg(t().tag_bg, 4.0)
}
/// The heart, in the theme's love colour when `on`; a click toggles (`Ui::event` looks for `id`).
fn heart(on: bool, id: u32) -> Node {
    let c = if on { t().love } else { t().dim };
    row(vec![icon(
        if on { icons::HEART_FILL } else { icons::HEART },
        14.0,
        c,
    )])
    .pad_xy(4.0, 4.0)
    .cursor(Cursor::Pointer)
    .id(id)
}
fn nav(icon_id: usize, label: &str, selected: bool, id: u32) -> Node {
    let c = if selected { t().pink_text } else { t().dim };
    let n = row(vec![
        icon(icon_id, 16.0, c),
        text(label, 13.0, c).w(Fill(1)).truncate(),
    ])
    .spacing(10.0)
    .align(Align::Center)
    .pad_xy(12.0, 0.0)
    .size(Fill(1), Fixed(34.0))
    .hover(t().hover)
    .cursor(Cursor::Pointer)
    .id(id);
    if selected { n.bg(t().nav_sel, 10.0) } else { n }
}
/// A file in the tree: in the sidebar it ends in "...", on hover the whole name is a bar that
/// reaches over the sidebar's edge.
fn track(name: &str, depth: f32, i: usize) -> Node {
    let bar = |label: Node, w: Length, bg: Color| {
        row(vec![icon(icons::MUSIC, 10.0, t().text), label])
            .spacing(5.0)
            .align(Align::Center)
            .pad_xy(6.0, 0.0)
            .size(w, Fixed(17.0))
            .bg(bg, 0.0)
    };
    row(vec![
        space().size(Fixed(37.0 + 14.0 * depth), Fixed(17.0)),
        bar(
            text(name, 12.5, t().text).w(Fill(1)).truncate(),
            Fill(1),
            t().track,
        )
        .hover(t().hover)
        .cursor(Cursor::Pointer)
        .id(TRACK0 + i as u32)
        .over(bar(text(name, 12.5, t().text), Shrink, t().hover)),
    ])
    .size(Fill(1), Fixed(17.0))
}
fn folder(d: &Dir, depth: f32, open: bool) -> Node {
    let (chevron, icon_id) = if open {
        (icons::CHEVRON_DOWN, icons::FOLDER_OPEN)
    } else {
        (icons::CHEVRON, icons::FOLDER)
    };
    row(vec![
        space().w(Fixed(20.0 + 14.0 * depth)),
        icon(chevron, 10.0, t().dim),
        icon(icon_id, 16.0, t().dim),
        text(d.name.as_str(), 13.0, t().text).w(Fill(1)).truncate(),
        text(d.count.to_string(), 10.0, t().dim)
            .pad_xy(6.0, 2.0)
            .bg(t().field, 9.0),
        space().w(Fixed(4.0)),
    ])
    .spacing(6.0)
    .align(Align::Center)
    .size(Fill(1), Fixed(21.0))
    .id(FOLDER0 + d.id)
    .cursor(Cursor::Pointer)
}
fn sidebar(h: u32, ui: &Ui) -> Node {
    let (mut tree, mut bar) = (vec![], space().w(Fixed(0.0)));
    match &*ui.tree.lock().unwrap() {
        None => tree.push(text("Scanning...", 11.0, t().dim).pad_xy(14.0, 4.0)),
        Some(scan) => {
            let (avail, rows) = (
                tree_avail(h, ui.sources.len()),
                tree_rows(&scan.roots, &ui.toggled),
            );
            let n = fit(&rows, ui.tree_first, avail);
            for (i, r) in rows.iter().enumerate().skip(ui.tree_first).take(n) {
                tree.push(match *r {
                    Row::Folder(d, depth, open) => folder(d, depth, open),
                    Row::Track(d, k, depth) => track(&d.files[k], depth, i),
                });
            }
            if let Some((last, th)) = tree_bar(&rows, avail) {
                let top = (avail - th) * ui.tree_first as f32 / last as f32;
                bar = col(vec![
                    space().h(Fixed(top)),
                    space().size(Fixed(6.0), Fixed(th)).bg(t().dim, 3.0),
                ])
                .align(Align::Center)
                .size(Fixed(12.0), Fixed(avail))
                .bg(t().field, 0.0)
                .id(TREE_SCROLL)
                .cursor(Cursor::Pointer);
            }
        }
    }
    col(vec![
        col(vec![
            nav(icons::HOUSE, "Library", ui.tab == Tab::Library, NAV_LIBRARY),
            nav(
                icons::HEART,
                "Favorites",
                ui.tab == Tab::Favorites,
                NAV_FAVORITES,
            ),
        ]
        .into_iter()
        .chain(ui.sources.iter().enumerate().map(|(i, s)| {
            nav(
                icons::GLOBE,
                &s.name,
                ui.tab == Tab::Plugin(i),
                NAV_PLUGIN0 + i as u32,
            )
        }))
        .collect())
        .spacing(2.0)
        .pad(8.0)
        .w(Fill(1)),
        text("FILE TREE", 10.0, t().dim).pad_xy(14.0, 8.0),
        row(vec![col(tree).spacing(0.0).size(Fill(1), Fill(1)), bar])
            .size(Fill(1), Fill(1))
            .id(TREE),
        col(vec![
            row(vec![
                space().w(Fill(1)),
                row(vec![
                    icon(icons::PLUS, 11.0, t().dim),
                    text("Add Folder", 11.0, t().dim),
                ])
                .spacing(4.0),
                space().w(Fill(1)),
            ])
            .align(Align::Center)
            .size(Fill(1), Fixed(28.0))
            .bg(t().field, 8.0)
            .border(t().dash, 1.0)
            .dashed()
            .hover(t().hover)
            .cursor(Cursor::Pointer)
            .id(ADD),
        ])
        .pad_xy(9.0, 8.0)
        .w(Fill(1)),
    ])
    .size(Fixed(220.0), Fill(1))
    .bg(t().panel, 0.0)
}
fn table(h: u32, ui: &Ui) -> Node {
    let head = |s: &str, w: Length| text(s, 10.0, t().dim).w(w);
    let (vis, total) = (visible_rows(h), ui.results.len());
    let mut rows = vec![];
    for (k, &i) in ui.results.iter().skip(ui.first).take(vis).enumerate() {
        let Some(f) = ui.files.get(i) else { continue };
        let lead = if f.source.is_some() {
            space().w(Fixed(22.0)) // a url has no heart
        } else {
            heart(ui.is_favorite(&f.path), HEART0 + k as u32)
        };
        rows.push(
            row(vec![
                row(vec![
                    lead,
                    text(f.name.as_str(), 13.0, t().text).w(Fill(1)).truncate(),
                ])
                .spacing(12.0)
                .align(Align::Center)
                .w(Fill(1)),
                text(f.ext(), 10.0, t().dim).w(Fixed(70.0)),
                text("-", 10.0, t().dim).w(Fixed(50.0)),
            ])
            .pad_xy(16.0, 0.0)
            .align(Align::Center)
            .size(Fill(1), Fixed(36.0))
            .hover(t().row_hover)
            .cursor(Cursor::Pointer)
            .id(ROW0 + k as u32),
        );
        rows.push(line());
    }
    if total == 0 {
        let none = match ui.tab {
            Tab::Favorites => "No favorites".to_string(),
            Tab::Plugin(i) => ui.sources.get(i).map_or_else(String::new, Source::message),
            Tab::Library => "No samples".to_string(),
        };
        rows.push(text(none, 12.0, t().dim).w(Fill(1)).truncate().pad(16.0));
    }
    let bar = if total > vis {
        col(vec![
            space().h(Fixed(ui.thumb_top(h))),
            space()
                .size(Fixed(6.0), Fixed(thumb_h(vis, total)))
                .bg(t().dim, 3.0),
        ])
        .align(Align::Center)
        .size(Fixed(12.0), Fixed(vis as f32 * ROW_H))
        .bg(t().field, 0.0)
        .id(SCROLL)
        .cursor(Cursor::Pointer)
    } else {
        space().w(Fixed(12.0))
    };
    col(vec![
        row(vec![
            head("NAME", Fill(1)),
            head("TYPE", Fixed(70.0)),
            head("BPM", Fixed(50.0)),
        ])
        .pad_xy(16.0, 0.0)
        .align(Align::Center)
        .size(Fill(1), Fixed(34.0)),
        line(),
        row(vec![col(rows).size(Fill(1), Fill(1)), bar])
            .size(Fill(1), Fill(1))
            .id(ROWS),
    ])
    .size(Fill(1), Fill(1))
}
fn lerp(a: Color, b: Color, k: f32) -> Color {
    [0, 1, 2, 3].map(|i| a[i] + (b[i] - a[i]) * k)
}
/// The decoded clip, one column per px of the window (up to `COLS_MAX`) as tall as its peak; the
/// part before the playhead (`pos`, in frames) is a gradient from the theme's `wave_a` (top of the
/// 82 px) to `wave_b` (bottom), the rest is `wave_off`.
fn waveform(w: u32, clip: &Clip, pos: usize) -> Node {
    let n = (w as usize).min(COLS_MAX);
    let cursor = (pos as u64 * n as u64 / clip.frames.len().max(1) as u64) as usize;
    let bars = clip
        .columns(n)
        .into_iter()
        .enumerate()
        .map(|(i, peak)| {
            if i == cursor {
                space().size(Fixed(1.0), Fill(1)).bg(t().playhead, 0.0)
            } else {
                let h = (peak * 78.0).clamp(1.0, 78.0);
                let bar = space().size(Fill(1), Fixed(h));
                if i < cursor {
                    // the bar is centred in the 82 px, so its ends sit at these shares of the
                    // gradient, which then runs through all the bars as one
                    let at = |y: f32| lerp(t().wave_a, t().wave_b, y / 82.0);
                    bar.gradient(at((82.0 - h) / 2.0), at((82.0 + h) / 2.0))
                } else {
                    bar.bg(t().wave_off, 0.0)
                }
            }
        })
        .collect();
    row(bars)
        .align(Align::Center)
        .size(Fill(1), Fixed(82.0))
        .id(WAVE)
        .cursor(Cursor::Pointer)
}
/// `secs` as `00:00`.
fn clock(secs: f32) -> String {
    let s = secs as u32;
    format!("{:02}:{:02}", s / 60, s % 60)
}
fn player(w: u32, ui: &Ui) -> Node {
    let note = |s: &str| {
        row(vec![
            space().w(Fill(1)),
            text(s, 12.0, t().dim),
            space().w(Fill(1)),
        ])
        .align(Align::Center)
        .size(Fill(1), Fixed(82.0))
    };
    let (head, wave, now, total) = match &ui.current {
        None => (
            vec![text("No sample selected", 13.0, t().dim)],
            note("Click a sample to play it"),
            0.0,
            0.0,
        ),
        Some(l) => {
            let lead = if l.remote {
                space().w(Fixed(22.0))
            } else {
                heart(ui.is_favorite(&l.path), PLAYER_HEART)
            };
            let head = vec![lead, text(l.name.as_str(), 13.0, t().text), chip(&l.ext)];
            match &l.clip {
                Some(c) => (
                    head,
                    waveform(w, c, ui.player.pos()),
                    ui.player.pos() as f32 / c.rate as f32,
                    c.secs(),
                ),
                None if l.loading => (
                    head,
                    note(if l.remote {
                        "Downloading..."
                    } else {
                        "Loading..."
                    }),
                    0.0,
                    0.0,
                ),
                None => (
                    head,
                    note(l.error.as_deref().unwrap_or("Cannot decode this file")),
                    0.0,
                    0.0,
                ),
            }
        }
    };
    // pause / play, only with a sample that decoded
    let live = matches!(&ui.current, Some(Loaded { clip: Some(_), .. }));
    let glyph = if ui.player.paused() {
        icons::PLAY
    } else {
        icons::PAUSE
    };
    let mut button = row(vec![icon(
        glyph,
        18.0,
        if live { t().pink_text } else { t().dim },
    )])
    .pad_xy(14.0, 8.0)
    .bg(t().field, 10.0)
    .border(t().line, 1.0);
    if live {
        button = button.id(PLAY).cursor(Cursor::Pointer);
    }
    col(vec![
        line(),
        row(head)
            .spacing(12.0)
            .align(Align::Center)
            .pad_xy(16.0, 0.0)
            .size(Fill(1), Fixed(32.0)),
        wave,
        row(vec![
            space().w(Fill(1)),
            text(clock(now), 12.0, t().text),
            button,
            text(clock(total), 12.0, t().text),
            space().w(Fill(1)),
        ])
        .spacing(16.0)
        .align(Align::Center)
        .size(Fill(1), Fixed(46.0)),
    ])
    .w(Fill(1))
}
fn status(ui: &Ui) -> Node {
    // a plugin tab counts its results, the others the scanned files (not the caches after them)
    let count = if matches!(ui.tab, Tab::Plugin(_)) {
        ui.files.len()
    } else {
        ui.lib.len()
    };
    row(vec![
        space().w(Fixed(16.0)),
        space().size(Fixed(6.0), Fixed(6.0)).bg(t().pink, 3.0),
        text(count.to_string(), 10.0, t().text),
        text("samples", 10.0, t().dim),
        space().size(Fixed(8.0), Fixed(1.0)),
        vline().h(Fixed(12.0)),
        space().size(Fixed(8.0), Fixed(1.0)),
        // a click switches between one shot and loop
        row(vec![
            text("Playback mode:", 10.0, t().dim),
            text(
                if ui.player.looping() {
                    "Loop"
                } else {
                    "One shot"
                },
                10.0,
                t().text,
            ),
        ])
        .spacing(5.0)
        .align(Align::Center)
        .pad_xy(6.0, 3.0)
        .bg(t().bg, 5.0)
        .hover(t().field)
        .cursor(Cursor::Pointer)
        .id(PLAYMODE),
        space().w(Fill(1)),
        // flush in the corner: this square is where the window takes the resize drag (`in_grip`)
        col(vec![
            space().h(Fill(1)),
            icon(icons::GRIP_ICON, GRIP / ui.scale(), t().dim),
        ])
        .size(Fixed(GRIP / ui.scale()), Fill(1))
        .cursor(Cursor::Resize),
    ])
    .spacing(5.0)
    .align(Align::Center)
    .size(Fill(1), Fixed(28.0))
}

pub fn view(w: u32, h: u32, ui: &Ui) -> Node {
    use_scheme(ui.settings.scheme);
    let placeholder = match ui.tab {
        Tab::Plugin(i) => ui.sources.get(i).map_or("Search samples".into(), |s| {
            format!("Search {} (press Enter)", s.name)
        }),
        _ => "Search samples".into(),
    };
    let search = row(vec![
        icon(icons::SEARCH, 14.0, t().dim),
        ui.search.node(&placeholder, 13.0, t().text, t().dim),
    ])
    .spacing(10.0)
    .align(Align::Center)
    .pad_xy(14.0, 0.0)
    .size(Fixed(420.0), Fixed(30.0))
    .bg(t().field, 15.0)
    .border(
        if ui.search.focused {
            t().pink
        } else {
            t().line
        },
        1.0,
    )
    .id(SEARCH)
    .cursor(Cursor::Text);
    let settings = row(vec![
        icon(icons::SETTINGS, 14.0, t().text),
        text("Settings", 12.0, t().text),
    ])
    .spacing(6.0)
    .align(Align::Center)
    .pad_xy(12.0, 0.0)
    .size(Shrink, Fixed(28.0))
    .bg(t().field, 14.0)
    .border(t().line, 1.0)
    .hover(t().hover)
    .cursor(Cursor::Pointer)
    .id(SETTINGS_BTN);
    let header = row(vec![
        row(vec![
            text("S", 14.0, t().pink),
            text("AMPLEVAULT", 11.0, t().dim),
        ])
        .spacing(1.0)
        .align(Align::Center),
        space().w(Fill(1)),
        search,
        space().w(Fill(1)),
        settings,
    ])
    .align(Align::Center)
    .pad_xy(18.0, 0.0)
    .size(Fill(1), Fixed(56.0));
    col(vec![
        header,
        line(),
        row(vec![sidebar(h, ui), vline(), table(h, ui)]).size(Fill(1), Fill(1)),
        player(w, ui),
        line(),
        status(ui),
    ])
    .fill()
    .bg(t().bg, 0.0)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::scan::Scan;

    /// A tree of one open folder at `dir` holding `good.wav` (a real 4 frame file) and `n - 1`
    /// more names, none on disk. Its rows: the folder, then the files.
    pub(crate) fn tree_of(dir: &std::path::Path, n: usize) -> Tree {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            dir.join("good.wav"),
            crate::audio::tests::wav(8000, &[0, 16384, -32768, 8192]),
        )
        .unwrap();
        let files = ["good.wav".to_string()]
            .into_iter()
            .chain((1..n).map(|i| format!("f{i:02}.wav")))
            .collect();
        let root = Dir {
            id: 0,
            name: "r".into(),
            path: dir.to_path_buf(),
            dirs: vec![],
            files,
            count: n,
        };
        Arc::new(std::sync::Mutex::new(Some(Scan {
            roots: vec![root],
            files: Arc::default(),
        })))
    }

    #[test]
    fn reserved_heights_match_the_layout() {
        let (w, h) = (920, 690);
        let ui = Ui::new(Tree::default(), Arc::default());
        let l = layout(
            &view(w, h, &ui),
            [0.0, 0.0, w as f32, h as f32],
            &bx_app::bx_shell::font::width,
            [f32::NAN; 2],
        );
        let area = |id| l.rects.iter().find(|r| r.0 == id).unwrap().1[3];
        assert_eq!(h as f32 - area(TREE), TREE_RESERVED);
        assert_eq!(h as f32 - area(ROWS), TABLE_RESERVED);
    }

    #[test]
    fn the_library_search_also_finds_what_the_plugins_cached() {
        let mk = |name: &str, path: &str, source| Sample {
            name: name.into(),
            path: path.into(),
            source,
            ..Default::default()
        };
        let mut ui = Ui::new(Tree::default(), Arc::default());
        *ui.tree.lock().unwrap() = Some(Scan {
            roots: vec![],
            files: Arc::new(vec![mk("kick.wav", "kick.wav", None)]),
        });
        let mut s = Source::fixed("p", vec![]);
        s.cache = Arc::new(vec![mk("Kick_Far.mp3", "https://x/k.mp3", Some(0))]);
        s.version = 1;
        ui.sources = vec![s];
        ui.search.text = "kick".into();
        ui.refresh();
        let mut both = ui.results.clone(); // the scanned file and the cached hit
        both.sort();
        assert_eq!(both, vec![0, 1]);
        let mut hearts: Vec<_> = (0..2).map(|k| ui.row_path(k).is_some()).collect();
        hearts.sort();
        assert_eq!(hearts, [false, true]); // a url has no heart, no drag
        ui.search.text = "far kick".into(); // words in any order
        ui.refresh();
        assert_eq!(ui.results, vec![1]);
        ui.search.text.clear(); // no query: only the scanned files, as before
        ui.refresh();
        assert_eq!((ui.results.clone(), ui.lib.len()), (vec![0], 1));
    }

    #[test]
    fn rows_tint_on_hover_and_a_click_loads_them_into_the_player() {
        let dir = std::env::temp_dir().join(format!("sm-select-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let sample = |name: &str, bytes: &[u8]| {
            std::fs::write(dir.join(name), bytes).unwrap();
            Sample {
                name: name.into(),
                path: dir.join(name),
                ..Default::default()
            }
        };
        let pcm = crate::audio::tests::wav(8000, &[0, 16384, -32768, 8192]);
        let mut ui = Ui::new(Tree::default(), Arc::default());
        ui.files = Arc::new(vec![sample("good.wav", &pcm), sample("bad.wav", b"nope")]);
        ui.results = vec![0, 1];
        let (w, h) = (920, 690);
        let lay = |ui: &Ui, m| {
            let area = [0.0, 0.0, w as f32, h as f32];
            layout(&view(w, h, ui), area, &bx_app::bx_shell::font::width, m)
        };
        let tinted = |l: &Layout, r: Rect| {
            l.base
                .iter()
                .any(|p| matches!(p, Prim::Quad(q, c, _) if *q == r && *c == t().row_hover))
        };
        let l = lay(&ui, [f32::NAN; 2]);
        let row1 = l.rects.iter().find(|r| r.0 == ROW0 + 1).unwrap().1;
        assert!(!tinted(&l, row1));
        assert!(tinted(&lay(&ui, [row1[0] + 5.0, row1[1] + 5.0]), row1));

        // what a press on a row picks up to drag out
        assert_eq!(ui.source_of(ROW0), Some(dir.join("good.wav")));
        assert_eq!(ui.source_of(ROW0 + 1), Some(dir.join("bad.wav")));
        assert_eq!((ui.source_of(ROW0 + 2), ui.source_of(SEARCH)), (None, None));
        assert_eq!((ui.source_of(FOLDER0), ui.source_of(TRACK0)), (None, None)); // no tree yet

        assert!(ui.current.is_none()); // the player starts empty
        let mut voice = crate::audio::Voice::new(ui.player.clone(), 8000.0);
        let (mut l, mut r) = ([1.0; 4], [1.0; 4]);
        ui.select(0);
        assert!(ui.current.as_ref().unwrap().loading); // decoding, nothing plays yet
        ui.settle();
        let c = ui.current.as_ref().unwrap();
        assert_eq!((c.name.as_str(), c.ext.as_str()), ("good.wav", "WAV"));
        assert_eq!(c.clip.as_ref().unwrap().frames.len(), 4);
        voice.render(&mut l, &mut r);
        assert_eq!(l, [0.0, 0.5, -1.0, 0.25]); // and the plugin plays it
        ui.select(1); // a file that does not decode: shown as such, the old sample stops
        ui.settle();
        assert!(ui.current.as_ref().unwrap().clip.is_none());
        voice.render(&mut l, &mut r);
        assert_eq!(l, [0.0; 4]);
        ui.select(7); // no such row
        assert_eq!(ui.current.as_ref().unwrap().name, "bad.wav");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_tree_scrolls_and_a_click_on_a_file_plays_it() {
        let dir = std::env::temp_dir().join(format!("sm-tree-{}", std::process::id()));
        let player = Arc::<Player>::default();
        let mut ui = Ui::new(tree_of(&dir, 30), player.clone());
        let h = 690;
        // 285 px of tree: the folder (21) and 15 files (17) fit, 31 rows in all
        let (last, th) = ui.tree_bar(h).unwrap();
        assert_eq!(last, 15);
        assert!(th < 285.0 && th >= THUMB_MIN);
        ui.rects = vec![(TREE_SCROLL, [200.0, 100.0, 12.0, 285.0])].into();
        ui.grab_tree(100.0 + 285.0, h); // on the track below the thumb: the end
        assert_eq!(ui.tree_first, 15);
        ui.drag_tree(-10_000.0, h);
        assert_eq!(ui.tree_first, 0);
        ui.tree_first = 999;
        ui.clamp(h);
        assert_eq!(ui.tree_first, 15);
        ui.toggled.insert(0); // closing the folder: one row, nothing to scroll
        ui.clamp(h);
        assert_eq!((ui.tree_first, ui.tree_bar(h)), (0, None));
        ui.toggled.clear();

        ui.select_track(0); // a folder: nothing
        ui.select_track(99); // no such row
        assert!(ui.current.is_none());
        assert_eq!(ui.source_of(TRACK0 + 1), Some(dir.join("good.wav")));
        assert_eq!(
            (ui.source_of(TRACK0), ui.source_of(TRACK0 + 99)),
            (None, None)
        ); // a folder, none
        ui.select_track(2); // f01.wav is not on disk: shown as such
        ui.settle();
        assert!(ui.current.as_ref().unwrap().clip.is_none());
        ui.select_track(1);
        ui.settle();
        let c = ui.current.as_ref().unwrap();
        assert_eq!((c.name.as_str(), c.ext.as_str()), ("good.wav", "WAV"));
        assert_eq!(c.clip.as_ref().unwrap().frames.len(), 4);

        // the pause button state and a click on the waveform
        ui.rects = vec![(WAVE, [100.0, 0.0, 200.0, 82.0])].into();
        ui.seek(250.0); // 3/4 of the way
        assert_eq!(player.pos(), 3);
        ui.seek(-50.0);
        assert_eq!(player.pos(), 0);
        ui.seek(10_000.0); // the end is the last frame
        assert_eq!(player.pos(), 3);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_thumb_follows_the_rows_and_the_drag_reaches_both_ends() {
        let h = 690;
        let vis = visible_rows(h);
        let mut ui = Ui::new(Tree::default(), Arc::default());
        ui.results = (0..100).collect();
        ui.rects = vec![(SCROLL, [0.0, 100.0, 12.0, vis as f32 * ROW_H])].into();
        assert_eq!(ui.thumb_top(h), 0.0);
        ui.grab(100.0 + thumb_h(vis, 100) / 2.0, h); // on the thumb
        assert_eq!(ui.first, 0);
        ui.drag_to(10_000.0, h);
        assert_eq!(ui.first, 100 - vis);
        assert_eq!(ui.thumb_top(h) + thumb_h(vis, 100), vis as f32 * ROW_H);
        ui.drag_to(-10_000.0, h);
        assert_eq!(ui.first, 0);
    }
}
