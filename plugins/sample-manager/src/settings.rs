//! The settings dialog, in tabs: General (colour scheme, UI scale, the tracked folders) and Plugins
//! (a card for each plugin file: a tick to load it, a trash can to delete it, and a button that
//! adds one). `dialog` is the tree
//! (`App::overlay`), `Ui::dialog_event` what a click in it does.
use crate::icons;
use crate::sources::Status;
use crate::state::{SCALES, Settings};
use crate::theme::{schemes, swatch, t, use_scheme};
use crate::view::{ADD, Job, SETTINGS_BTN, SettingsTab, Ui};
use bx_app::bx_shell::input::{Event, Key};
use bx_app::bx_ui::*;

/// `Layout::rects` ids of the dialog (see `SETTINGS_BTN`).
pub(crate) const CLOSE: u32 = SETTINGS_BTN + 1;
pub(crate) const CARD: u32 = SETTINGS_BTN + 2;
pub(crate) const SCHEME0: u32 = SETTINGS_BTN + 100;
pub(crate) const SCALE0: u32 = SETTINGS_BTN + 200;
const REMOVE0: u32 = SETTINGS_BTN + 300;
pub(crate) const TAB_GENERAL: u32 = SETTINGS_BTN + 10;
pub(crate) const TAB_PLUGINS: u32 = SETTINGS_BTN + 11;
pub(crate) const ADD_PLUGIN: u32 = SETTINGS_BTN + 20;
/// The tick and the trash can of the plugin card `i` are `TOGGLE0 + i` and `TRASH0 + i`.
pub(crate) const TOGGLE0: u32 = SETTINGS_BTN + 10_000;
pub(crate) const TRASH0: u32 = SETTINGS_BTN + 20_000;
/// Folders and plugin cards shown at a time; the wheel scrolls the rest.
const SHOWN: usize = 6;
const SHOWN_PLUGINS: usize = 4;

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
        self.set_settings_tab(SettingsTab::General); // opens on the first tab
    }

    fn set_settings_tab(&mut self, tab: SettingsTab) {
        (self.settings_tab, self.dlg_first) = (tab, 0);
    }

    /// Ticks or unticks the plugin card `i`: loads or unloads the plugin (the tick follows the
    /// click at once, the plugins are loaded again behind it).
    fn toggle_plugin(&mut self, i: usize) {
        if let Some((file, on)) = self.plugin_files.get_mut(i) {
            *on = !*on;
            let job = Job::EnablePlugin(file.clone(), *on);
            self.jobs.lock().unwrap().push(job);
        }
    }

    /// Deletes the plugin of the card `i`.
    fn delete_plugin(&mut self, i: usize) {
        if i < self.plugin_files.len() {
            let (file, _) = self.plugin_files.remove(i);
            self.jobs.lock().unwrap().push(Job::RemovePlugin(file));
        }
    }

    fn change(&mut self, f: impl FnOnce(&mut Settings)) {
        f(&mut self.settings);
        self.settings_to_save = Some(self.settings.clone());
    }

    /// Keeps the first row of the tab's list (folders or plugins) inside the list.
    pub fn clamp_dialog(&mut self) {
        let (n, shown) = match self.settings_tab {
            SettingsTab::General => (self.folders().map_or(0, |f| f.len()), SHOWN),
            SettingsTab::Plugins => (self.plugin_files.len(), SHOWN_PLUGINS),
        };
        self.dlg_first = self.dlg_first.min(n.saturating_sub(shown));
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
                        TAB_GENERAL => self.set_settings_tab(SettingsTab::General),
                        TAB_PLUGINS => self.set_settings_tab(SettingsTab::Plugins),
                        ADD => self.jobs.lock().unwrap().push(Job::AddFolder),
                        ADD_PLUGIN => self.jobs.lock().unwrap().push(Job::AddPlugin),
                        i if (SCHEME0..SCALE0).contains(&i) => {
                            self.change(|s| s.scheme = (i - SCHEME0) as usize)
                        }
                        i if (SCALE0..REMOVE0).contains(&i) => {
                            if let Some(&p) = SCALES.get((i - SCALE0) as usize) {
                                self.change(|s| s.scale = p);
                            }
                        }
                        i if (TOGGLE0..TRASH0).contains(&i) => {
                            self.toggle_plugin((i - TOGGLE0) as usize)
                        }
                        i if i >= TRASH0 => self.delete_plugin((i - TRASH0) as usize),
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

/// The General tab: colour scheme, UI scale, folders.
fn general(ui: &Ui) -> Node {
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
    col(vec![
        section("COLOR SCHEME", schemes),
        section("UI SCALE", scales),
        section(
            "FOLDERS",
            col(vec![col(list).spacing(4.0), row(vec![add])]).spacing(8.0),
        ),
    ])
    .spacing(18.0)
}

/// What the plugin card `i` says: its name and description, the hosts it may call, why it did not
/// load; an unloaded plugin is only a file name.
fn plugin_card(ui: &Ui, i: usize, file: &str, on: bool) -> Node {
    let p = t();
    let src = ui.sources.iter().find(|s| s.file == file).filter(|_| on);
    let stem = std::path::Path::new(file)
        .file_stem()
        .map_or_else(|| file.to_string(), |s| s.to_string_lossy().into_owned());
    let name = src.map_or(stem, |s| s.name.clone());
    let about = match src {
        None if !on => "Unloaded".to_string(),
        Some(s) if !s.description.is_empty() => s.description.clone(),
        Some(s) => match &s.status {
            Status::Failed(e) => e.clone(),
            _ => String::new(),
        },
        None => String::new(),
    };
    let network = match src {
        Some(s) if s.hosts.is_empty() => "No network access".to_string(),
        Some(s) => format!("Network: {}", s.hosts.join(", ")),
        None => String::new(),
    };
    let thumb = match src.and_then(|s| s.slot) {
        Some(cell) => image(cell, 44.0, 44.0),
        // no icon: it is not loaded, or it did not load
        None => row(vec![icon(icons::GLOBE, 20.0, p.dim)])
            .align(Align::Center)
            .pad_xy(12.0, 0.0)
            .size(Fixed(44.0), Fixed(44.0))
            .bg(p.panel, 8.0),
    };
    let tick = row(if on {
        vec![icon(icons::CHECK, 12.0, p.pink)]
    } else {
        vec![]
    })
    .align(Align::Center)
    .pad_xy(5.0, 0.0)
    .size(Fixed(22.0), Fixed(22.0))
    .bg(p.panel, 5.0)
    .border(if on { p.pink } else { p.line }, 1.0)
    .hover(p.hover)
    .cursor(Cursor::Pointer)
    .id(TOGGLE0 + i as u32);
    let trash = row(vec![icon(icons::TRASH, 12.0, p.dim)])
        .align(Align::Center)
        .pad_xy(8.0, 0.0)
        .size(Fixed(28.0), Fixed(28.0))
        .bg(p.panel, 6.0)
        .hover(p.hover)
        .cursor(Cursor::Pointer)
        .id(TRASH0 + i as u32);
    row(vec![
        tick,
        thumb,
        col(vec![
            text(name, 13.0, if on { p.text } else { p.dim })
                .w(Fill(1))
                .truncate(),
            text(about, 11.0, p.dim).w(Fill(1)).truncate(),
            text(network, 10.0, p.dim).w(Fill(1)).truncate(),
        ])
        .spacing(2.0)
        .w(Fill(1)),
        trash,
    ])
    .spacing(12.0)
    .align(Align::Center)
    .pad_xy(12.0, 0.0)
    .size(Fill(1), Fixed(64.0))
    .bg(p.field, 8.0)
}

/// The Plugins tab: the cards and the button that adds a plugin.
fn plugins(ui: &Ui) -> Node {
    let p = t();
    let n = ui.plugin_files.len();
    let mut list: Vec<Node> = ui
        .plugin_files
        .iter()
        .enumerate()
        .skip(ui.dlg_first)
        .take(SHOWN_PLUGINS)
        .map(|(i, (file, on))| plugin_card(ui, i, file, *on))
        .collect();
    if n == 0 {
        list.push(text("No plugins", 11.0, p.dim).pad_xy(10.0, 6.0));
    } else if n > SHOWN_PLUGINS {
        let last = (ui.dlg_first + SHOWN_PLUGINS).min(n);
        list.push(
            text(
                format!("{}-{} of {}", ui.dlg_first + 1, last, n),
                10.0,
                p.dim,
            )
            .pad_xy(10.0, 2.0),
        );
    }
    let add = chip(
        vec![
            icon(icons::PLUS, 11.0, p.dim),
            text("Add Plugin", 11.0, p.text),
        ],
        false,
        ADD_PLUGIN,
    );
    col(vec![
        section("PLUGINS", col(list).spacing(6.0)),
        row(vec![add]),
    ])
    .spacing(12.0)
}

/// The dialog over a dimmed window.
pub fn dialog(ui: &Ui) -> Node {
    use_scheme(ui.settings.scheme);
    let p = t();
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
        row(vec![
            chip(
                vec![text("General", 11.0, p.text)],
                ui.settings_tab == SettingsTab::General,
                TAB_GENERAL,
            ),
            chip(
                vec![text("Plugins", 11.0, p.text)],
                ui.settings_tab == SettingsTab::Plugins,
                TAB_PLUGINS,
            ),
        ])
        .spacing(8.0),
        match ui.settings_tab {
            SettingsTab::General => general(ui),
            SettingsTab::Plugins => plugins(ui),
        },
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
