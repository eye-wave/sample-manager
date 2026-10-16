//! The screen, as a `bx_ui` tree (sample data for now). `view` knows the window size only for the
//! number of waveform bars; everything else is the layout engine's job.
use crate::audio::{Clip, Player, decode};
use crate::icons::{self, GRIP};
use crate::scan::{Dir, Sample, Tree};
use crate::search::search;
use crate::theme::*;
use bx_ui::*;
use std::collections::HashSet;
use std::sync::Arc;

/// `Layout::rects` id of the search box.
pub const SEARCH: u32 = 1;
/// `Layout::rects` id of the folder `Dir::id` is `id - FOLDER0`.
pub const FOLDER0: u32 = 1000;
/// `Layout::rects` id of the file tree's area and of the table rows' area (`TREE_RESERVED` and
/// `TABLE_RESERVED` are checked against them in the tests).
pub const TREE: u32 = 2;
pub const ROWS: u32 = 3;
/// `Layout::rects` id of the table's scrollbar (the whole track).
pub const SCROLL: u32 = 4;
/// `Layout::rects` id of the table row `k` (the k-th shown) is `ROW0 + k`; below `FOLDER0`.
pub const ROW0: u32 = 10;
/// Height of everything in the window except the file tree (it cannot scroll, so rows that do not
/// fit are left out).
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

/// The sample in the player.
pub struct Loaded {
    pub name: String,
    pub ext: String,
    /// `None`: it could not be decoded.
    pub clip: Option<Arc<Clip>>,
}

/// What the screen remembers between frames.
pub struct Ui {
    pub search: TextBox,
    /// Where the named nodes were put last frame, to tell what a click hit.
    pub rects: Vec<(u32, Rect)>,
    /// The tracked folders, filled in by the scan thread.
    pub tree: Tree,
    /// Folders (`Dir::id`) flipped from their default: roots start open, everything else closed.
    pub toggled: HashSet<u32>,
    /// The scanned files and what the search box found in them (indices into `files`).
    pub files: Arc<Vec<Sample>>,
    pub query: String,
    pub results: Vec<usize>,
    /// The first table row shown (an index into `results`).
    pub first: usize,
    /// A scrollbar drag: where on the thumb (px from its top) it was grabbed.
    pub drag: Option<f32>,
    /// The player: empty until a table row is clicked.
    pub current: Option<Loaded>,
    /// Where the plugin plays the sample (nothing does in the standalone app).
    pub player: Arc<Player>,
}
impl Ui {
    pub fn new(tree: Tree, player: Arc<Player>) -> Self {
        Self {
            search: TextBox::new(""),
            rects: vec![],
            tree,
            toggled: HashSet::new(),
            files: Arc::default(),
            query: String::new(),
            results: vec![],
            first: 0,
            drag: None,
            current: None,
            player,
        }
    }

    /// A click on the `k`-th shown table row: that sample replaces the one in the player and the
    /// plugin plays it (one that does not decode silences the old one).
    pub fn select(&mut self, k: usize) {
        let Some(&i) = self.results.get(self.first + k) else {
            return;
        };
        let f = &self.files[i];
        let clip = decode(&f.path).ok().map(Arc::new);
        self.player.play(clip.clone().unwrap_or_default());
        self.current = Some(Loaded {
            name: f.name.clone(),
            ext: f.ext(),
            clip,
        });
    }

    /// Searches again if the scan has finished or the search text changed.
    pub fn refresh(&mut self) {
        let files = match &*self.tree.lock().unwrap() {
            Some(s) => s.files.clone(),
            None => return,
        };
        if Arc::ptr_eq(&files, &self.files) && self.search.text == self.query {
            return;
        }
        self.results = search(&files, &self.search.text);
        self.files = files;
        self.query = self.search.text.clone();
        self.first = 0;
    }

    /// Keeps the table from scrolling past its last row (also after a resize).
    pub fn clamp(&mut self, h: u32) {
        self.first = self
            .first
            .min(self.results.len().saturating_sub(visible_rows(h)));
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

    fn track_top(&self) -> Option<f32> {
        self.rects.iter().find(|r| r.0 == SCROLL).map(|r| r.1[1])
    }

    /// A click at `y` on the scrollbar: the thumb is grabbed where it is, or the track click
    /// centres it there; either way the drag goes on until `Event::Release`.
    pub fn grab(&mut self, y: f32, h: u32) {
        let Some(ty) = self.track_top() else { return };
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
        let (Some(off), Some(ty)) = (self.drag, self.track_top()) else {
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
    space().size(Fill(1), Fixed(1.0)).bg(LINE, 0.0)
}
fn vline() -> Node {
    space().size(Fixed(1.0), Fill(1)).bg(LINE, 0.0)
}
fn chip(s: &str) -> Node {
    text(s, 10.0, CHIP_TEXT).pad_xy(7.0, 3.0).bg(PINK, 4.0)
}
fn nav(icon_id: usize, label: &str, selected: bool) -> Node {
    let c = if selected { PINK_TEXT } else { DIM };
    let n = row(vec![icon(icon_id, 16.0, c), text(label, 13.0, c)])
        .spacing(10.0)
        .align(Align::Center)
        .pad_xy(12.0, 0.0)
        .size(Fill(1), Fixed(34.0));
    if selected { n.bg(NAV_SEL, 10.0) } else { n }
}
/// A file in the tree: in the sidebar it ends in "...", on hover the whole name is a bar that
/// reaches over the sidebar's edge.
fn track(name: &str, depth: f32) -> Node {
    let bar = |t: Node, w: Length, bg: Color| {
        row(vec![icon(icons::MUSIC, 10.0, TEXT), t])
            .spacing(5.0)
            .align(Align::Center)
            .pad_xy(6.0, 0.0)
            .size(w, Fixed(17.0))
            .bg(bg, 0.0)
    };
    row(vec![
        space().size(Fixed(37.0 + 14.0 * depth), Fixed(17.0)),
        bar(text(name, 12.5, TEXT).w(Fill(1)).truncate(), Fill(1), TRACK)
            .hover(HOVER)
            .cursor(Cursor::Pointer)
            .over(bar(text(name, 12.5, TEXT), Shrink, HOVER)),
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
        icon(chevron, 10.0, DIM),
        icon(icon_id, 16.0, DIM),
        text(d.name.as_str(), 13.0, TEXT).w(Fill(1)).truncate(),
        text(d.count.to_string(), 10.0, DIM)
            .pad_xy(6.0, 2.0)
            .bg(FIELD, 9.0),
        space().w(Fixed(4.0)),
    ])
    .spacing(6.0)
    .align(Align::Center)
    .size(Fill(1), Fixed(21.0))
    .id(FOLDER0 + d.id)
    .cursor(Cursor::Pointer)
}
/// The rows of `d` and, if it is open, of everything in it, with their heights.
fn rows(d: &Dir, depth: f32, ui: &Ui, out: &mut Vec<(Node, f32)>) {
    let open = (depth == 0.0) != ui.toggled.contains(&d.id);
    out.push((folder(d, depth, open), 21.0));
    if open {
        d.dirs.iter().for_each(|c| rows(c, depth + 1.0, ui, out));
        out.extend(d.files.iter().map(|f| (track(f, depth), 17.0)));
    }
}
fn sidebar(h: u32, ui: &Ui) -> Node {
    let mut tree = vec![];
    match &*ui.tree.lock().unwrap() {
        None => tree.push(text("Scanning...", 11.0, DIM).pad_xy(14.0, 4.0)),
        Some(scan) => {
            let mut all = vec![];
            scan.roots.iter().for_each(|r| rows(r, 0.0, ui, &mut all));
            let mut used = 0.0;
            for (n, height) in all {
                used += height;
                if used > h as f32 - TREE_RESERVED {
                    break; // no scrolling yet, the rest is cut off
                }
                tree.push(n);
            }
        }
    }
    col(vec![
        col(vec![
            nav(icons::HOUSE, "Library", true),
            nav(icons::HEART, "Favorites", false),
        ])
        .spacing(2.0)
        .pad(8.0)
        .w(Fill(1)),
        text("FILE TREE", 10.0, DIM).pad_xy(14.0, 8.0),
        col(tree)
            .spacing(0.0)
            .pad_xy(0.0, 0.0)
            .size(Fill(1), Fill(1))
            .id(TREE),
        col(vec![
            row(vec![
                space().w(Fill(1)),
                row(vec![
                    icon(icons::PLUS, 11.0, DIM),
                    text("Add Folder", 11.0, DIM),
                ])
                .spacing(4.0),
                space().w(Fill(1)),
            ])
            .align(Align::Center)
            .size(Fill(1), Fixed(28.0))
            .bg(FIELD, 8.0)
            .border(DASH, 1.0)
            .dashed(),
        ])
        .pad_xy(9.0, 8.0)
        .w(Fill(1)),
    ])
    .size(Fixed(220.0), Fill(1))
    .bg(PANEL, 0.0)
}
fn table(h: u32, ui: &Ui) -> Node {
    let head = |s: &str, w: Length| text(s, 10.0, DIM).w(w);
    let (vis, total) = (visible_rows(h), ui.results.len());
    let mut rows = vec![];
    for (k, &i) in ui.results.iter().skip(ui.first).take(vis).enumerate() {
        let f = &ui.files[i];
        rows.push(
            row(vec![
                row(vec![
                    icon(icons::HEART, 14.0, DIM),
                    text(f.name.as_str(), 13.0, TEXT).w(Fill(1)).truncate(),
                ])
                .spacing(12.0)
                .align(Align::Center)
                .w(Fill(1)),
                text(f.ext(), 10.0, DIM).w(Fixed(70.0)),
                text("-", 10.0, DIM).w(Fixed(50.0)),
            ])
            .pad_xy(16.0, 0.0)
            .align(Align::Center)
            .size(Fill(1), Fixed(36.0))
            .hover(ROW_HOVER)
            .cursor(Cursor::Pointer)
            .id(ROW0 + k as u32),
        );
        rows.push(line());
    }
    if total == 0 {
        rows.push(text("No samples", 12.0, DIM).pad(16.0));
    }
    let bar = if total > vis {
        col(vec![
            space().h(Fixed(ui.thumb_top(h))),
            space()
                .size(Fixed(6.0), Fixed(thumb_h(vis, total)))
                .bg(DIM, 3.0),
        ])
        .align(Align::Center)
        .size(Fixed(12.0), Fixed(vis as f32 * ROW_H))
        .bg(FIELD, 0.0)
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
/// The decoded clip, one column per px of the window (up to `COLS_MAX`) as tall as its peak; the
/// part before the playhead (`pos`, in frames) is pink.
fn waveform(w: u32, clip: &Clip, pos: usize) -> Node {
    let n = (w as usize).min(COLS_MAX);
    let cursor = pos * n / clip.frames.len().max(1);
    let bars = clip
        .columns(n)
        .into_iter()
        .enumerate()
        .map(|(i, peak)| {
            if i == cursor {
                space().size(Fixed(1.0), Fill(1)).bg(PLAYHEAD, 0.0)
            } else {
                let c = if i < cursor { PINK } else { WAVE_OFF };
                space()
                    .size(Fill(1), Fixed((peak * 78.0).clamp(1.0, 78.0)))
                    .bg(c, 0.0)
            }
        })
        .collect();
    row(bars).align(Align::Center).size(Fill(1), Fixed(82.0))
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
            text(s, 12.0, DIM),
            space().w(Fill(1)),
        ])
        .align(Align::Center)
        .size(Fill(1), Fixed(82.0))
    };
    let (head, wave, now, total) = match &ui.current {
        None => (
            vec![text("No sample selected", 13.0, DIM)],
            note("Click a sample to play it"),
            0.0,
            0.0,
        ),
        Some(l) => {
            let head = vec![
                icon(icons::HEART, 14.0, DIM),
                text(l.name.as_str(), 13.0, TEXT),
                chip(&l.ext),
            ];
            match &l.clip {
                Some(c) => (
                    head,
                    waveform(w, c, ui.player.pos()),
                    ui.player.pos() as f32 / c.rate as f32,
                    c.secs(),
                ),
                None => (head, note("Cannot decode this file"), 0.0, 0.0),
            }
        }
    };
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
            text(clock(now), 12.0, TEXT),
            row(vec![icon(
                icons::PAUSE,
                18.0,
                if ui.current.is_some() { PINK_TEXT } else { DIM },
            )])
            .pad_xy(14.0, 8.0)
            .bg(FIELD, 10.0)
            .border(LINE, 1.0),
            text(clock(total), 12.0, TEXT),
            space().w(Fill(1)),
        ])
        .spacing(16.0)
        .align(Align::Center)
        .size(Fill(1), Fixed(46.0)),
    ])
    .w(Fill(1))
}
fn status(ui: &Ui) -> Node {
    row(vec![
        space().w(Fixed(16.0)),
        space().size(Fixed(6.0), Fixed(6.0)).bg(PINK, 3.0),
        text(ui.files.len().to_string(), 10.0, TEXT),
        text("samples", 10.0, DIM),
        space().size(Fixed(8.0), Fixed(1.0)),
        vline().h(Fixed(12.0)),
        space().size(Fixed(8.0), Fixed(1.0)),
        text("Playback mode:", 10.0, DIM),
        text("Loop", 10.0, TEXT),
        space().w(Fill(1)),
        // flush in the corner: this square is where the window takes the resize drag (`in_grip`)
        col(vec![space().h(Fill(1)), icon(icons::GRIP_ICON, GRIP, DIM)])
            .size(Fixed(GRIP), Fill(1))
            .cursor(Cursor::Resize),
    ])
    .spacing(5.0)
    .align(Align::Center)
    .size(Fill(1), Fixed(28.0))
}

pub fn view(w: u32, h: u32, ui: &Ui) -> Node {
    let search = row(vec![
        icon(icons::SEARCH, 14.0, DIM),
        ui.search.node("Search samples", 13.0, TEXT, DIM),
    ])
    .spacing(10.0)
    .align(Align::Center)
    .pad_xy(14.0, 0.0)
    .size(Fixed(420.0), Fixed(30.0))
    .bg(FIELD, 15.0)
    .border(if ui.search.focused { PINK } else { LINE }, 1.0)
    .id(SEARCH)
    .cursor(Cursor::Text);
    let settings = row(vec![
        icon(icons::SETTINGS, 14.0, TEXT),
        text("Settings", 12.0, TEXT),
    ])
    .spacing(6.0)
    .align(Align::Center)
    .pad_xy(12.0, 0.0)
    .size(Shrink, Fixed(28.0))
    .bg(FIELD, 14.0)
    .border(LINE, 1.0);
    let header = row(vec![
        row(vec![text("S", 14.0, PINK), text("AMPLEVAULT", 11.0, DIM)])
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
    .bg(BG, 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_heights_match_the_layout() {
        let (w, h) = (920, 690);
        let ui = Ui::new(Tree::default(), Arc::default());
        let l = layout(
            &view(w, h, &ui),
            [0.0, 0.0, w as f32, h as f32],
            &bx_shell::font::width,
            [f32::NAN; 2],
        );
        let area = |id| l.rects.iter().find(|r| r.0 == id).unwrap().1[3];
        assert_eq!(h as f32 - area(TREE), TREE_RESERVED);
        assert_eq!(h as f32 - area(ROWS), TABLE_RESERVED);
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
            }
        };
        let pcm = crate::audio::tests::wav(8000, &[0, 16384, -32768, 8192]);
        let mut ui = Ui::new(Tree::default(), Arc::default());
        ui.files = Arc::new(vec![sample("good.wav", &pcm), sample("bad.wav", b"nope")]);
        ui.results = vec![0, 1];
        let (w, h) = (920, 690);
        let lay = |ui: &Ui, m| {
            let area = [0.0, 0.0, w as f32, h as f32];
            layout(&view(w, h, ui), area, &bx_shell::font::width, m)
        };
        let tinted = |l: &Layout, r: Rect| {
            l.base
                .iter()
                .any(|p| matches!(p, Prim::Quad(q, c, _) if *q == r && *c == ROW_HOVER))
        };
        let l = lay(&ui, [f32::NAN; 2]);
        let row1 = l.rects.iter().find(|r| r.0 == ROW0 + 1).unwrap().1;
        assert!(!tinted(&l, row1));
        assert!(tinted(&lay(&ui, [row1[0] + 5.0, row1[1] + 5.0]), row1));

        assert!(ui.current.is_none()); // the player starts empty
        let mut voice = crate::audio::Voice::new(ui.player.clone(), 8000.0);
        let (mut l, mut r) = ([1.0; 4], [1.0; 4]);
        ui.select(0);
        let c = ui.current.as_ref().unwrap();
        assert_eq!((c.name.as_str(), c.ext.as_str()), ("good.wav", "WAV"));
        assert_eq!(c.clip.as_ref().unwrap().frames.len(), 4);
        voice.render(&mut l, &mut r);
        assert_eq!(l, [0.0, 0.5, -1.0, 0.25]); // and the plugin plays it
        ui.select(1); // a file that does not decode: shown as such, the old sample stops
        assert!(ui.current.as_ref().unwrap().clip.is_none());
        voice.render(&mut l, &mut r);
        assert_eq!(l, [0.0; 4]);
        ui.select(7); // no such row
        assert_eq!(ui.current.as_ref().unwrap().name, "bad.wav");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_thumb_follows_the_rows_and_the_drag_reaches_both_ends() {
        let h = 690;
        let vis = visible_rows(h);
        let mut ui = Ui::new(Tree::default(), Arc::default());
        ui.results = (0..100).collect();
        ui.rects = vec![(SCROLL, [0.0, 100.0, 12.0, vis as f32 * ROW_H])];
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
