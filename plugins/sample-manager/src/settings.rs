//! The settings dialog: colour scheme, UI scale and the tracked folders. `dialog` is the tree
//! (`App::overlay`), `Ui::dialog_event` what a click in it does.
use crate::icons;
use crate::state::{SCALES, Settings};
use crate::theme::{schemes, swatch, t, use_scheme};
use crate::view::{ADD, Job, SETTINGS_BTN, Ui};
use bx_app::bx_shell::input::{Event, Key};
use bx_app::bx_ui::*;

/// `Layout::rects` ids of the dialog (see `SETTINGS_BTN`).
pub(crate) const CLOSE: u32 = SETTINGS_BTN + 1;
pub(crate) const CARD: u32 = SETTINGS_BTN + 2;
pub(crate) const SCHEME0: u32 = SETTINGS_BTN + 100;
pub(crate) const SCALE0: u32 = SETTINGS_BTN + 200;
const REMOVE0: u32 = SETTINGS_BTN + 300;
/// Folders shown at a time; the wheel scrolls the rest.
const SHOWN: usize = 6;

fn section(title: &str, body: Node) -> Node {
    col(vec![text(title, 10.0, t().dim), body]).spacing(8.0)
}

/// A choice in a row of choices: highlighted while it is the current one.
fn chip(kids: Vec<Node>, selected: bool, id: u32) -> Node {
    row(kids)
        .spacing(6.0)
        .align(Align::Center)
        .pad_xy(10.0, 0.0)
        .size(Shrink, Fixed(28.0))
        .bg(t().field, 14.0)
        .border(if selected { t().pink } else { t().line }, 1.0)
        .hover(t().hover)
        .cursor(Cursor::Pointer)
        .id(id)
}

impl Ui {
    /// The paths of the tracked folders (what the last scan was given); `None` while it scans.
    fn folders(&self) -> Option<Vec<std::path::PathBuf>> {
        let g = self.tree.lock().unwrap();
        Some(g.as_ref()?.roots.iter().map(|d| d.path.clone()).collect())
    }

    pub fn open_settings(&mut self) {
        self.settings_open = true;
        self.search.focused = false;
    }

    fn change(&mut self, f: impl FnOnce(&mut Settings)) {
        f(&mut self.settings);
        self.settings_to_save = Some(self.settings.clone());
    }

    /// Keeps the folder list's first row inside the list.
    pub fn clamp_dialog(&mut self) {
        let n = self.folders().map_or(0, |f| f.len());
        self.dlg_first = self.dlg_first.min(n.saturating_sub(SHOWN));
    }

    /// An event while the dialog is open: everything below it is out of reach.
    pub fn dialog_event(&mut self, e: Event) {
        match e {
            Event::Click(x, y) => {
                let at = [x as f32, y as f32];
                let hit: Vec<u32> = self.rects.at(at).collect();
                if !hit.contains(&CARD) || hit.contains(&CLOSE) {
                    self.settings_open = false; // outside the dialog, or its X
                }
                for id in hit {
                    match id {
                        ADD => self.jobs.lock().unwrap().push(Job::AddFolder),
                        i if (SCHEME0..SCALE0).contains(&i) => {
                            self.change(|s| s.scheme = (i - SCHEME0) as usize)
                        }
                        i if (SCALE0..REMOVE0).contains(&i) => {
                            if let Some(&p) = SCALES.get((i - SCALE0) as usize) {
                                self.change(|s| s.scale = p);
                            }
                        }
                        i if i >= REMOVE0 => {
                            let path = self
                                .folders()
                                .and_then(|f| f.get((i - REMOVE0) as usize).cloned());
                            if let Some(p) = path {
                                self.jobs.lock().unwrap().push(Job::Remove(p));
                            }
                        }
                        _ => {}
                    }
                }
            }
            Event::Scroll(n) => {
                self.dlg_first = self.dlg_first.saturating_add_signed(n as isize);
                self.clamp_dialog();
            }
            Event::Key(Key::Escape) => self.settings_open = false,
            Event::Release => (self.drag, self.tree_drag) = (None, None),
            _ => {}
        }
    }
}

/// The dialog over a dimmed window.
pub fn dialog(ui: &Ui) -> Node {
    use_scheme(ui.settings.scheme);
    let p = t();
    let schemes = row(schemes()
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let (accent, _) = swatch(i);
            chip(
                vec![
                    space().size(Fixed(12.0), Fixed(12.0)).bg(accent, 6.0),
                    text(s.name, 11.0, p.text),
                ],
                i == ui.settings.scheme,
                SCHEME0 + i as u32,
            )
        })
        .collect())
    .spacing(8.0);
    let scales = row(SCALES
        .iter()
        .enumerate()
        .map(|(i, s)| {
            chip(
                vec![text(format!("{s}%"), 11.0, p.text)],
                *s == ui.settings.scale,
                SCALE0 + i as u32,
            )
        })
        .collect())
    .spacing(8.0);
    let mut list = vec![];
    match ui.folders() {
        None => list.push(text("Scanning...", 11.0, p.dim).pad_xy(10.0, 6.0)),
        Some(f) if f.is_empty() => list.push(text("No folders", 11.0, p.dim).pad_xy(10.0, 6.0)),
        Some(f) => {
            for (i, path) in f.iter().enumerate().skip(ui.dlg_first).take(SHOWN) {
                let remove = row(vec![icon(icons::X, 10.0, p.dim)])
                    .align(Align::Center)
                    .pad_xy(6.0, 0.0)
                    .size(Fixed(22.0), Fixed(22.0))
                    .bg(p.field, 4.0)
                    .hover(p.hover)
                    .cursor(Cursor::Pointer)
                    .id(REMOVE0 + i as u32);
                list.push(
                    row(vec![
                        icon(icons::FOLDER, 14.0, p.dim),
                        text(path.display().to_string(), 12.0, p.text)
                            .w(Fill(1))
                            .truncate(),
                        remove,
                    ])
                    .spacing(8.0)
                    .align(Align::Center)
                    .pad_xy(10.0, 0.0)
                    .size(Fill(1), Fixed(32.0))
                    .bg(p.field, 6.0),
                );
            }
            if f.len() > SHOWN {
                let last = (ui.dlg_first + SHOWN).min(f.len());
                list.push(
                    text(
                        format!("{}-{} of {}", ui.dlg_first + 1, last, f.len()),
                        10.0,
                        p.dim,
                    )
                    .pad_xy(10.0, 2.0),
                );
            }
        }
    }
    let add = chip(
        vec![
            icon(icons::PLUS, 11.0, p.dim),
            text("Add Folder", 11.0, p.text),
        ],
        false,
        ADD,
    );
    let close = row(vec![icon(icons::X, 12.0, p.dim)])
        .align(Align::Center)
        .pad_xy(8.0, 0.0)
        .size(Fixed(28.0), Fixed(28.0))
        .bg(p.field, 14.0)
        .hover(p.hover)
        .cursor(Cursor::Pointer)
        .id(CLOSE);
    let card = col(vec![
        row(vec![
            text("Settings", 15.0, p.text),
            space().w(Fill(1)),
            close,
        ])
        .align(Align::Center)
        .size(Fill(1), Fixed(28.0)),
        section("COLOR SCHEME", schemes),
        section("UI SCALE", scales),
        section(
            "FOLDERS",
            col(vec![col(list).spacing(4.0), row(vec![add])]).spacing(8.0),
        ),
    ])
    .spacing(18.0)
    .pad(20.0)
    .size(Fixed(500.0), Shrink)
    .bg(p.panel, 12.0)
    .border(p.line, 1.0)
    .id(CARD);
    col(vec![
        space().h(Fill(1)),
        // Fill wide: a Shrink row would be as wide as the card and sit at the left
        row(vec![space().w(Fill(1)), card, space().w(Fill(1))]).w(Fill(1)),
        space().h(Fill(1)),
    ])
    .fill()
    .bg([0.0, 0.0, 0.0, 0.6], 0.0)
}
