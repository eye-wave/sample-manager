//! The GUI: what is drawn for a given window size. No Vulkan or windowing here, `shell::gpu` draws
//! what [`scene`] returns. Shared by the standalone app and the plugin. The tree is view.rs, the
//! layout is `bx_ui`; this file only turns the draw list into vertices.
use crate::audio::Player;
use crate::icons::ICONS;
use crate::scan::{Tree, scan};
use crate::state::{
    add_dir, favorites, remove_dir, save_favorites, save_settings, settings, tracked_dirs,
};
use crate::view::{
    ADD, FOLDER0, HEART0, Job, NAV_FAVORITES, NAV_LIBRARY, PLAY, PLAYER_HEART, PLAYMODE, ROW0,
    SCROLL, SEARCH, SETTINGS_BTN, TRACK0, TREE, TREE_SCROLL, Tab, Ui, WAVE, view,
};
use bx_app::bx_shell::font;
use bx_app::bx_shell::gpu::{self, Content, Scene, Target};
use bx_app::bx_shell::input::{Event, Events};
use bx_app::bx_ui::{Layout, Node, RectExt, Rects};
use bx_app::{App, Mesh, Typing, frame};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex};

pub use bx_app::bx_shell::gpu::{NO_MOUSE, pack_mouse, pack_size};

/// Initial window size in px.
pub const W: u32 = 920;
pub const H: u32 = 690;
/// Smallest allowed window size, the same shape as the initial one (the waveform and the table
/// need about this much).
pub const MIN_W: u32 = 860;
pub const MIN_H: u32 = MIN_W * H / W;

/// The window keeps the aspect ratio of `W` x `H`: the height that goes with width `w`.
pub const fn height_for(w: u32) -> u32 {
    w * H / W
}

impl App for Ui {
    const ICONS: &'static [Mesh] = &ICONS;

    fn view(&self, (w, h): (u32, u32)) -> Node {
        view(w, h, self)
    }

    fn rects_mut(&mut self) -> &mut Rects {
        &mut self.rects
    }

    fn scale(&self) -> f32 {
        Ui::scale(self)
    }

    fn overlay(&self, _: (u32, u32)) -> Option<Node> {
        self.settings_open.then(|| crate::settings::dialog(self))
    }

    fn event(&mut self, e: Event, (_, h): (u32, u32), mouse: [f32; 2]) {
        if self.settings_open {
            return self.dialog_event(e);
        }
        if self.search.feed(&e) {
            return;
        }
        match e {
            Event::Click(x, y) => {
                let at = [x as f32, y as f32];
                let hit: Vec<u32> = self.rects.at(at).collect();
                if hit.contains(&SETTINGS_BTN) {
                    return self.open_settings();
                }
                // hearts and tabs sit on top of rows: they take the click alone
                if let Some(&id) = hit.iter().find(|&&id| (HEART0..PLAYER_HEART).contains(&id)) {
                    if let Some(p) = self.row_path((id - HEART0) as usize) {
                        self.toggle_favorite(&p);
                    }
                    return;
                }
                if hit.contains(&PLAYER_HEART) {
                    if let Some(p) = self.current.as_ref().map(|l| l.path.clone()) {
                        self.toggle_favorite(&p);
                    }
                    return;
                }
                if hit.contains(&NAV_LIBRARY) || hit.contains(&NAV_FAVORITES) {
                    self.tab = if hit.contains(&NAV_FAVORITES) {
                        Tab::Favorites
                    } else {
                        Tab::Library
                    };
                    return;
                }
                self.search.focused = hit.contains(&SEARCH);
                if let Some(&id) = hit.iter().find(|&&id| (ROW0..FOLDER0).contains(&id)) {
                    self.select((id - ROW0) as usize);
                }
                if let Some(&id) = hit.iter().find(|&&id| id >= TRACK0) {
                    self.select_track((id - TRACK0) as usize);
                }
                if hit.contains(&SCROLL) {
                    self.grab(at[1], h);
                }
                if hit.contains(&TREE_SCROLL) {
                    self.grab_tree(at[1], h);
                }
                if hit.contains(&PLAY) {
                    self.player.toggle_pause();
                }
                if hit.contains(&ADD) {
                    self.jobs.lock().unwrap().push(Job::AddFolder);
                }
                if hit.contains(&PLAYMODE) {
                    self.player.toggle_loop();
                }
                if hit.contains(&WAVE) {
                    self.seek(at[0]);
                }
                if let Some(&id) = hit.iter().find(|&&id| (FOLDER0..TRACK0).contains(&id)) {
                    let id = id - FOLDER0;
                    if !self.toggled.remove(&id) {
                        self.toggled.insert(id);
                    }
                }
            }
            // the wheel scrolls what the mouse is over: the tree, else the table
            Event::Scroll(n) => {
                let n = n as isize * 3;
                if self.rects.hit(TREE, mouse) {
                    self.tree_first = self.tree_first.saturating_add_signed(n);
                } else {
                    self.first = self.first.saturating_add_signed(n);
                }
            }
            Event::Release => (self.drag, self.tree_drag) = (None, None),
            _ => {}
        }
    }

    fn before_view(&mut self, (_, h): (u32, u32), mouse: [f32; 2]) {
        self.refresh();
        self.clamp(h);
        self.clamp_dialog();
        if !mouse[1].is_nan() {
            // the thumb follows the mouse
            self.drag_to(mouse[1], h);
            self.drag_tree(mouse[1], h);
        }
    }

    fn after_layout(&mut self, l: &Layout, mouse: [f32; 2]) {
        // a press on a row or a file in the tree would pick that file up
        self.hover_src = l
            .rects
            .iter()
            .rev()
            .find(|&&(id, r)| ((ROW0..FOLDER0).contains(&id) || id >= TRACK0) && r.hit(mouse))
            .and_then(|&(id, _)| self.source_of(id));
    }
}

/// What is drawn in a `w` x `h` px window with the mouse at `mouse` (px, NaN if it is elsewhere),
/// after the `events` (clicks and typing) went to `ui`.
pub fn scene(w: u32, h: u32, mouse: [f32; 2], ui: &mut Ui, events: Vec<Event>) -> Scene {
    frame(ui, (w, h), mouse, 0.0, events)
}

/// Draws the GUI on `target` until `stop` is set: [`gpu::render`] with this module's scene and
/// the font atlas, see there for the arguments (`mouse` is `pack_mouse`d, `NO_MOUSE` when off the
/// window; `events` is what the window got, clicks and typing); `player` is where a clicked sample
/// goes to be played. Run it on a thread of its own.
pub fn run(
    target: &impl Target,
    size: &AtomicU64,
    stop: &AtomicBool,
    presented: &AtomicU64,
    mouse: &AtomicU64,
    events: &Events,
    player: &Arc<Player>,
) {
    let atlas = font::atlas();
    let tree = Tree::default();
    let ui = Mutex::new(Ui::new(tree.clone(), player.clone()));
    {
        let mut u = ui.lock().unwrap();
        (u.settings, u.favorites) = (settings(), favorites());
    }
    let jobs = ui.lock().unwrap().jobs.clone();
    let (tree, ui) = (&tree, &ui); // so the `move` closure below copies the references
    std::thread::scope(|s| {
        // the scan runs beside the renderer and wakes it when done; `stop` cuts it short. Then it
        // does what the clicks ask (`Job`) and saves the settings: the folder picker blocks, so it
        // lives here and not in the renderer
        s.spawn(move || {
            let rescan = || {
                *tree.lock().unwrap() = Some(scan(&tracked_dirs(), stop));
                events.wake();
            };
            rescan();
            while !stop.load(Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(33));
                let mut changed = false;
                for job in std::mem::take(&mut *jobs.lock().unwrap()) {
                    match job {
                        Job::AddFolder => {
                            if let Some(dir) = bx_app::bx_shell::dialog::pick_folder(stop) {
                                add_dir(&dir);
                                changed = true;
                            }
                        }
                        Job::Remove(dir) => {
                            remove_dir(&dir);
                            changed = true;
                        }
                    }
                }
                if changed {
                    *tree.lock().unwrap() = None; // "Scanning..." until it is done
                    events.wake();
                    rescan();
                }
                let (save, favs) = {
                    let mut u = ui.lock().unwrap();
                    (u.settings_to_save.take(), u.fav_to_save.take())
                };
                if let Some(s) = save {
                    save_settings(&s);
                }
                if let Some(f) = favs {
                    save_favorites(&f);
                }
            }
        });
        let busy = ui.lock().unwrap().busy.clone();
        // the playhead moves and decodes finish without any input: draw again when the position
        // changed, while a file decodes and once more when that ends
        s.spawn(move || {
            let (mut last, mut was_busy) = (0, 0);
            while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(33));
                let (p, b) = (player.pos(), busy.load(std::sync::atomic::Ordering::SeqCst));
                if p != last || b != 0 || b != was_busy {
                    (last, was_busy) = (p, b);
                    events.wake();
                }
            }
        });
        let content = Content {
            atlas: &atlas,
            scene: &|w, h, m| {
                let mut ui = ui.lock().unwrap();
                let s = scene(w, h, m, &mut ui, events.take());
                events.set_drag_source(ui.hover_src.take()); // for the window to drag out
                s
            },
            mouse,
            events,
        };
        gpu::render(target, size, stop, presented, &content)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::Sample;

    /// A `Ui` with one table row, a.wav (4 frames), the file already gone from disk.
    fn ui_with_a_row() -> Ui {
        // one file per call: the tests run in parallel and each removes its own
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("sm-render-{}-{n}.wav", std::process::id()));
        std::fs::write(
            &path,
            crate::audio::tests::wav(8000, &[0, 16384, -32768, 8192]),
        )
        .unwrap();
        let mut ui = Ui::new(Tree::default(), Arc::default());
        ui.files = Arc::new(vec![Sample {
            name: "a.wav".into(),
            path: path.clone(),
        }]);
        ui.results = vec![0];
        ui.select(0);
        ui.settle(); // the decode thread reads the file
        std::fs::remove_file(&path).unwrap();
        ui
    }

    /// `gpu` panics past 65536 vertices in a scene (all four lists together).
    #[test]
    fn a_loaded_waveform_stays_within_the_vertex_budget_at_any_width() {
        let mut ui = ui_with_a_row();
        for w in [MIN_W, W, 3000, 8192] {
            let s = scene(w, height_for(w), [f32::NAN; 2], &mut ui, vec![]);
            let n = s.flat.len() + s.textured.len() + s.over_flat.len() + s.over_textured.len();
            assert!(n <= 65536, "{w}: {n}");
        }
    }

    #[test]
    fn the_wheel_scrolls_the_tree_under_the_mouse_and_clicks_start_and_pause() {
        let dir = std::env::temp_dir().join(format!("sm-wheel-{}", std::process::id()));
        let player = Arc::<Player>::default();
        let mut ui = Ui::new(crate::view::tests::tree_of(&dir, 30), player.clone());
        let none = [f32::NAN; 2];
        scene(W, H, none, &mut ui, vec![]); // a frame first: it records where things are
        let rect = |ui: &Ui, id| ui.rects.iter().find(|r| r.0 == id).unwrap().1;
        let t = rect(&ui, TREE);
        let on_tree = [t[0] + 20.0, t[1] + 20.0];
        scene(W, H, on_tree, &mut ui, vec![Event::Scroll(2)]);
        assert_eq!((ui.tree_first, ui.first), (6, 0));
        scene(W, H, none, &mut ui, vec![Event::Scroll(1)]); // elsewhere: the table's
        assert_eq!(ui.tree_first, 6);

        // a click in the middle-left of the node `id`
        let at = |ui: &Ui, id| {
            let r = rect(ui, id);
            Event::Click((r[0] + 40.0) as i32, (r[1] + 5.0) as i32)
        };
        ui.tree_first = 0;
        scene(W, H, none, &mut ui, vec![]);
        let click = at(&ui, crate::view::TRACK0 + 1); // good.wav, the folder is row 0
        scene(W, H, none, &mut ui, vec![click]);
        assert_eq!(ui.current.as_ref().unwrap().name, "good.wav");
        ui.settle();
        assert!(!player.paused());

        scene(W, H, none, &mut ui, vec![]); // the player now has a play button and a waveform
        let click = at(&ui, PLAY);
        scene(W, H, none, &mut ui, vec![click]);
        assert!(player.paused());
        let w = rect(&ui, WAVE);
        let click = Event::Click((w[0] + w[2] / 2.0) as i32, (w[1] + 5.0) as i32);
        scene(W, H, none, &mut ui, vec![click]);
        assert_eq!(player.pos(), 2); // the middle of 4 frames
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn clicking_the_playback_mode_switches_between_one_shot_and_loop() {
        let mut ui = ui_with_a_row();
        let none = [f32::NAN; 2];
        scene(W, H, none, &mut ui, vec![]); // a frame first: it records where things are
        let r = ui.rects.iter().find(|r| r.0 == PLAYMODE).unwrap().1;
        let click = Event::Click((r[0] + 10.0) as i32, (r[1] + 5.0) as i32);
        assert!(!ui.player.looping()); // one shot by default
        scene(W, H, none, &mut ui, vec![click]);
        assert!(ui.player.looping());
        scene(W, H, none, &mut ui, vec![click]);
        assert!(!ui.player.looping());
    }

    #[test]
    fn clicking_a_table_row_puts_its_sample_in_the_player() {
        let mut ui = ui_with_a_row();
        ui.current = None; // as if nothing was picked yet
        let (w, h) = (W, H);
        scene(w, h, [f32::NAN; 2], &mut ui, vec![]); // a frame first: it records where the rows are
        let r = ui.rects.iter().find(|r| r.0 == ROW0).unwrap().1;
        let click = Event::Click((r[0] + 40.0) as i32, (r[1] + 10.0) as i32);
        scene(w, h, [f32::NAN; 2], &mut ui, vec![click]);
        assert_eq!(ui.current.as_ref().unwrap().name, "a.wav");
    }

    #[test]
    fn clicking_add_folder_asks_for_the_picker() {
        let mut ui = Ui::new(Tree::default(), Arc::default());
        let none = [f32::NAN; 2];
        scene(W, H, none, &mut ui, vec![]); // a frame first: it records where things are
        let r = ui.rects.rect(ADD).unwrap();
        let click = Event::Click((r[0] + 10.0) as i32, (r[1] + 5.0) as i32);
        assert!(ui.jobs.lock().unwrap().is_empty());
        scene(W, H, none, &mut ui, vec![click]);
        assert!(matches!(ui.jobs.lock().unwrap()[..], [Job::AddFolder]));
    }

    #[test]
    fn the_settings_dialog_changes_scheme_and_scale_and_closes() {
        use crate::settings::{CARD, CLOSE, SCALE0, SCHEME0};
        let mut ui = Ui::new(Tree::default(), Arc::default());
        let none = [f32::NAN; 2];
        // a click on node `id`; the window px are the logical ones times the scale
        let click = |ui: &Ui, id| {
            let r = ui.rects.rect(id).unwrap().map(|v| v * ui.scale());
            Event::Click((r[0] + 5.0) as i32, (r[1] + 5.0) as i32)
        };
        scene(W, H, none, &mut ui, vec![]);
        let open = click(&ui, SETTINGS_BTN);
        scene(W, H, none, &mut ui, vec![open]);
        assert!(ui.settings_open);
        scene(W, H, none, &mut ui, vec![]);
        assert!(ui.rects.rect(CARD).is_some() && ui.rects.rect(ROW0).is_none()); // modal
        let [x, y, w, h] = ui.rects.rect(CARD).unwrap();
        let (cx, cy) = (x + w / 2.0, y + h / 2.0); // centred in the window
        assert!(
            (cx - W as f32 / 2.0).abs() < 1.0 && (cy - H as f32 / 2.0).abs() < 1.0,
            "{cx} {cy}"
        );
        let (scheme, scale) = (click(&ui, SCHEME0 + 1), click(&ui, SCALE0 + 2));
        scene(W, H, none, &mut ui, vec![scheme, scale]);
        assert_eq!((ui.settings.scheme, ui.settings.scale), (1, 125));
        assert_eq!(ui.settings_to_save, Some(ui.settings.clone()));
        let s = scene(W, H, none, &mut ui, vec![]); // laid out at 125 %
        assert!(
            s.flat.len() + s.textured.len() + s.over_flat.len() + s.over_textured.len() <= 65536
        );
        let close = click(&ui, CLOSE);
        scene(W, H, none, &mut ui, vec![close]);
        assert!(!ui.settings_open);
        ui.open_settings();
        scene(W, H, none, &mut ui, vec![]);
        scene(W, H, none, &mut ui, vec![Event::Click(1, 1)]); // outside the card
        assert!(!ui.settings_open);
    }

    #[test]
    fn hearts_favor_files_and_the_favorites_tab_searches_only_those() {
        use crate::scan::{Sample, Scan};
        let mk = |n: &str| Sample {
            name: n.into(),
            path: n.into(),
        };
        let mut ui = Ui::new(Tree::default(), Arc::default());
        let all = vec![mk("kick.wav"), mk("snare.wav"), mk("kick2.wav")];
        *ui.tree.lock().unwrap() = Some(Scan {
            roots: vec![],
            files: Arc::new(all),
        });
        let none = [f32::NAN; 2];
        let click = |ui: &Ui, id| {
            let r = ui.rects.rect(id).unwrap();
            Event::Click((r[0] + 5.0) as i32, (r[1] + 5.0) as i32)
        };
        scene(W, H, none, &mut ui, vec![]);
        assert_eq!(ui.results.len(), 3);
        let heart = click(&ui, HEART0 + 2); // kick2.wav
        scene(W, H, none, &mut ui, vec![heart]);
        assert!(ui.is_favorite("kick2.wav".as_ref()) && ui.current.is_none()); // not played
        assert_eq!(ui.fav_to_save.as_ref().map(|f| f.len()), Some(1));
        let tab = click(&ui, NAV_FAVORITES);
        scene(W, H, none, &mut ui, vec![tab]);
        assert_eq!(ui.results, vec![2]);
        ui.search.text = "kick".into(); // "kick" matches both files, the tab keeps one
        scene(W, H, none, &mut ui, vec![]);
        assert_eq!(ui.results, vec![2]);
        ui.search.text = "snare".into();
        scene(W, H, none, &mut ui, vec![]);
        assert!(ui.results.is_empty());
        ui.search.text.clear();
        scene(W, H, none, &mut ui, vec![]);
        let heart = click(&ui, HEART0); // un-favor it: it leaves the tab
        scene(W, H, none, &mut ui, vec![heart]);
        scene(W, H, none, &mut ui, vec![]);
        assert!(ui.results.is_empty());
        let tab = click(&ui, NAV_LIBRARY);
        scene(W, H, none, &mut ui, vec![tab]);
        assert_eq!(ui.results.len(), 3);
    }
}
