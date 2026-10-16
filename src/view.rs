//! The screen, as a `bx_ui` tree (sample data for now). `view` knows the window size only for the
//! number of waveform bars; everything else is the layout engine's job.
use crate::icons::{self, GRIP};
use crate::theme::*;
use bx_ui::*;

/// `Layout::rects` id of the search box.
pub const SEARCH: u32 = 1;

/// What the screen remembers between frames.
pub struct Ui {
    pub search: TextBox,
    /// Where the named nodes were put last frame, to tell what a click hit.
    pub rects: Vec<(u32, Rect)>,
}
impl Ui {
    pub fn new() -> Self {
        Self {
            search: TextBox::new("warped"),
            rects: vec![],
        }
    }
}

/// folder, number of files, the files shown (a folder with files is open)
const TREE: [(&str, &str, &[&str]); 4] = [
    ("Other", "61", &[]),
    ("Rap", "10", &[]),
    (
        "Trap",
        "3",
        &[
            "Mine All Day (Instrumental) PewDiePie x Party In Backyard.mp3",
            "[FREE] Polo G ft. Juice WRLD Type Beat - Hurt.mp3",
            "[FREE] Sad Juice WRLD Type Beat - Lonely.mp3",
        ],
    ),
    ("Tropical House", "1", &["He's a Pirate DJ AG Remix.mp3"]),
];
/// name, favourite, tags
const ROWS: [(&str, bool, &[&str]); 5] = [
    ("timewarped-02.wav", false, &["sounddesign", "wavetable"]),
    ("timewarped-00.wav", false, &["sounddesign", "wavetable"]),
    ("timewarped-01.wav", true, &["sounddesign", "wavetable"]),
    (
        "Deep Fried Censor Buzz Sound Effect..mp3",
        false,
        &["sfx", "buzz"],
    ),
    (
        "M3PH_bass_one_yoinky_D.wav",
        false,
        &["bass", "yoy", "yoink"],
    ),
];
const SELECTED: usize = 2;

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
fn track(name: &str) -> Node {
    let bar = |t: Node, w: Length, bg: Color| {
        row(vec![icon(icons::MUSIC, 10.0, TEXT), t])
            .spacing(5.0)
            .align(Align::Center)
            .pad_xy(6.0, 0.0)
            .size(w, Fixed(17.0))
            .bg(bg, 0.0)
    };
    row(vec![
        space().size(Fixed(37.0), Fixed(17.0)),
        bar(text(name, 12.5, TEXT).w(Fill(1)).truncate(), Fill(1), TRACK)
            .hover(HOVER)
            .cursor(Cursor::Pointer)
            .over(bar(text(name, 12.5, TEXT), Shrink, HOVER)),
    ])
    .size(Fill(1), Fixed(17.0))
}
fn folder(name: &str, count: &str, open: bool) -> Node {
    let (chevron, icon_id) = if open {
        (icons::CHEVRON_DOWN, icons::FOLDER_OPEN)
    } else {
        (icons::CHEVRON, icons::FOLDER)
    };
    row(vec![
        space().w(Fixed(20.0)),
        icon(chevron, 10.0, DIM),
        icon(icon_id, 16.0, DIM),
        text(name, 13.0, TEXT),
        space().w(Fill(1)),
        text(count, 10.0, DIM).pad_xy(6.0, 2.0).bg(FIELD, 9.0),
        space().w(Fixed(4.0)),
    ])
    .spacing(6.0)
    .align(Align::Center)
    .size(Fill(1), Fixed(21.0))
}
fn sidebar() -> Node {
    let mut tree = vec![];
    for (name, n, files) in TREE {
        tree.push(folder(name, n, !files.is_empty()));
        tree.extend(files.iter().map(|f| track(f)));
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
            .size(Fill(1), Fill(1)),
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
fn table() -> Node {
    let head = |s: &str, w: Length| text(s, 10.0, DIM).w(w);
    let mut rows = vec![
        row(vec![
            head("NAME", Fill(1)),
            head("TYPE", Fixed(70.0)),
            head("BPM", Fixed(50.0)),
            head("TAGS", Fixed(170.0)),
        ])
        .pad_xy(16.0, 0.0)
        .align(Align::Center)
        .size(Fill(1), Fixed(34.0)),
        line(),
    ];
    for (i, (name, fav, tags)) in ROWS.iter().enumerate() {
        let heart = icon(icons::HEART, 14.0, if *fav { HEART } else { DIM });
        let r = row(vec![
            row(vec![heart, text(*name, 13.0, TEXT)])
                .spacing(12.0)
                .align(Align::Center)
                .w(Fill(1)),
            text("One-shot", 10.0, DIM).w(Fixed(70.0)),
            text("-", 10.0, DIM).w(Fixed(50.0)),
            row(tags.iter().map(|t| chip(t)).collect())
                .spacing(4.0)
                .w(Fixed(170.0)),
        ])
        .pad_xy(16.0, 0.0)
        .align(Align::Center)
        .size(Fill(1), Fixed(36.0));
        rows.push(if i == SELECTED { r.bg(FIELD, 0.0) } else { r });
        rows.push(line());
    }
    col(rows).size(Fill(1), Fill(1))
}
fn waveform(w: u32) -> Node {
    let n = w as usize / 3;
    let cursor = n * 236 / 1000;
    let bars = (0..n)
        .map(|i| {
            let x = i as f32;
            let a = 8.0
                + 52.0
                    * ((x * 0.37).sin() * (x * 0.11 + 1.0).sin()).abs().max(0.35)
                    * (0.6 + 0.4 * (x * 1.7).sin().abs());
            let c = if i < cursor { PINK } else { WAVE_OFF };
            if i == cursor {
                space().size(Fixed(1.0), Fill(1)).bg(PLAYHEAD, 0.0)
            } else {
                space().size(Fill(1), Fixed(a)).bg(c, 0.0)
            }
        })
        .collect();
    row(bars)
        .spacing(1.0)
        .align(Align::Center)
        .size(Fill(1), Fixed(82.0))
}
fn player(w: u32) -> Node {
    col(vec![
        line(),
        row(vec![
            icon(icons::HEART, 14.0, HEART),
            text("timewarped-01.wav", 13.0, TEXT),
            chip("sounddesign"),
            chip("wavetable"),
        ])
        .spacing(12.0)
        .align(Align::Center)
        .pad_xy(16.0, 0.0)
        .size(Fill(1), Fixed(32.0)),
        waveform(w),
        row(vec![
            space().w(Fill(1)),
            text("00:01", 12.0, TEXT),
            row(vec![icon(icons::PAUSE, 18.0, PINK_TEXT)])
                .pad_xy(14.0, 8.0)
                .bg(FIELD, 10.0)
                .border(LINE, 1.0),
            text("00:05", 12.0, TEXT),
            space().w(Fill(1)),
        ])
        .spacing(16.0)
        .align(Align::Center)
        .size(Fill(1), Fixed(46.0)),
    ])
    .w(Fill(1))
}
fn status() -> Node {
    row(vec![
        space().w(Fixed(16.0)),
        space().size(Fixed(6.0), Fixed(6.0)).bg(PINK, 3.0),
        text("15", 10.0, TEXT),
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

pub fn view(w: u32, ui: &Ui) -> Node {
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
        row(vec![sidebar(), vline(), table()]).size(Fill(1), Fill(1)),
        player(w),
        line(),
        status(),
    ])
    .fill()
    .bg(BG, 0.0)
}
