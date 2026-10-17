//! The GUI: what is drawn for a given window size. No Vulkan or windowing here, `shell::gpu` draws
//! what [`scene`] returns. Shared by the standalone app and the plugin. The tree is view.rs, the
//! layout is `bx_ui`; this file only turns the draw list into vertices.
use crate::audio::Player;
use crate::icons::ICONS;
use crate::scan::{Tree, scan};
use crate::state::tracked_dirs;
use crate::view::{FOLDER0, PLAY, ROW0, SCROLL, SEARCH, TRACK0, TREE, TREE_SCROLL, Ui, WAVE, view};
use bx_shell::font;
use bx_shell::gpu::{self, Content, Scene, Target, Vert};
use bx_shell::input::{Event, Events, Key};
use bx_ui::{Cursor, Edit, Prim, Rect, layout, quad_tris};
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, Mutex};

pub use bx_shell::gpu::{NO_MOUSE, pack_mouse, pack_size};

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

fn within([x, y, w, h]: Rect, p: [f32; 2]) -> bool {
    (x..x + w).contains(&p[0]) && (y..y + h).contains(&p[1])
}

/// What is drawn in a `w` x `h` px window with the mouse at `mouse` (px, NaN if it is elsewhere),
/// after the `events` (clicks and typing) went to `ui`.
pub fn scene(w: u32, h: u32, mouse: [f32; 2], ui: &mut Ui, events: Vec<Event>) -> Scene {
    for e in events {
        match e {
            Event::Click(x, y) => {
                let at = [x as f32, y as f32];
                let hit: Vec<u32> = ui
                    .rects
                    .iter()
                    .filter(|&&(_, r)| within(r, at))
                    .map(|&(id, _)| id)
                    .collect();
                ui.search.focused = hit.contains(&SEARCH);
                if let Some(&id) = hit.iter().find(|&&id| (ROW0..FOLDER0).contains(&id)) {
                    ui.select((id - ROW0) as usize);
                }
                if let Some(&id) = hit.iter().find(|&&id| id >= TRACK0) {
                    ui.select_track((id - TRACK0) as usize);
                }
                if hit.contains(&SCROLL) {
                    ui.grab(at[1], h);
                }
                if hit.contains(&TREE_SCROLL) {
                    ui.grab_tree(at[1], h);
                }
                if hit.contains(&PLAY) {
                    ui.player.toggle_pause();
                }
                if hit.contains(&WAVE) {
                    ui.seek(at[0]);
                }
                if let Some(&id) = hit.iter().find(|&&id| (FOLDER0..TRACK0).contains(&id)) {
                    let id = id - FOLDER0;
                    if !ui.toggled.remove(&id) {
                        ui.toggled.insert(id);
                    }
                }
            }
            // the wheel scrolls what the mouse is over: the tree, else the table
            Event::Scroll(n) => {
                let n = n as isize * 3;
                if ui
                    .rects
                    .iter()
                    .any(|&(id, r)| id == TREE && within(r, mouse))
                {
                    ui.tree_first = ui.tree_first.saturating_add_signed(n);
                } else {
                    ui.first = ui.first.saturating_add_signed(n);
                }
            }
            Event::Release => (ui.drag, ui.tree_drag) = (None, None),
            Event::Char(c) if ui.search.focused => ui.search.edit(Edit::Char(c)),
            Event::Key(Key::Enter | Key::Escape) => ui.search.focused = false,
            Event::Key(k) if ui.search.focused => {
                let edit = match k {
                    Key::Backspace => Edit::Backspace,
                    Key::Delete => Edit::Delete,
                    Key::CtrlBackspace => Edit::WordBackspace,
                    Key::CtrlDelete => Edit::WordDelete,
                    Key::CtrlA => Edit::SelectAll,
                    Key::Left => Edit::Left,
                    Key::Right => Edit::Right,
                    Key::Home => Edit::Home,
                    _ => Edit::End,
                };
                ui.search.edit(edit);
            }
            _ => {}
        }
    }
    ui.refresh();
    ui.clamp(h);
    if !mouse[1].is_nan() {
        // the thumb follows the mouse
        ui.drag_to(mouse[1], h);
        ui.drag_tree(mouse[1], h);
    }
    let (fw, fh) = (w as f32, h as f32);
    let ndc = |[x, y]: [f32; 2], c: [f32; 4]| -> Vert {
        [
            x / fw * 2.0 - 1.0,
            y / fh * 2.0 - 1.0,
            0.0,
            0.0,
            c[0],
            c[1],
            c[2],
            c[3],
        ]
    };
    // the prims of one layer, as flat and textured vertices
    let layer =
        |prims: Vec<Prim>| {
            let (mut flat, mut textured) = (vec![], vec![]);
            for p in prims {
                match p {
                    Prim::Quad(r, c, radius) => {
                        let mut t = vec![];
                        quad_tris(&mut t, r, radius);
                        flat.extend(t.into_iter().map(|v| ndc(v, c)));
                    }
                    Prim::Icon(id, [x, y, iw, ih], c) => {
                        let (v, idx) = ICONS[id];
                        flat.extend(idx.iter().map(|&i| {
                            ndc([x + v[i as usize][0] * iw, y + v[i as usize][1] * ih], c)
                        }));
                    }
                    Prim::Text {
                        x,
                        y,
                        size,
                        color,
                        s,
                    } => {
                        let mut t = vec![];
                        font::text(&mut t, &s, (x, y), size, (w, h));
                        textured.extend(t.into_iter().map(|[x, y, u, v]| {
                            [x, y, u, v, color[0], color[1], color[2], color[3]]
                        }));
                    }
                }
            }
            (flat, textured)
        };
    let l = layout(&view(w, h, ui), [0.0, 0.0, fw, fh], &font::width, mouse);
    // a press on a row or a file in the tree would pick that file up
    ui.hover_src = l
        .rects
        .iter()
        .rev()
        .find(|&&(id, r)| ((ROW0..FOLDER0).contains(&id) || id >= TRACK0) && within(r, mouse))
        .and_then(|&(id, _)| ui.source_of(id));
    ui.rects = l.rects.clone();
    let ((flat, textured), (over_flat, over_textured)) = (layer(l.base), layer(l.over));
    Scene {
        flat,
        textured,
        over_flat,
        over_textured,
        cursor: match l.cursor {
            Cursor::Default => gpu::CURSOR_ARROW,
            Cursor::Pointer => gpu::CURSOR_HAND,
            Cursor::Text => gpu::CURSOR_TEXT,
            Cursor::Resize => gpu::CURSOR_RESIZE,
        },
    }
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
    // the scan runs beside the renderer and wakes it when done; `stop` cuts it short
    std::thread::scope(|s| {
        s.spawn(|| {
            *tree.lock().unwrap() = Some(scan(&tracked_dirs(), stop));
            events.wake();
        });
        let ui = Mutex::new(Ui::new(tree.clone(), player.clone()));
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
        let path = std::env::temp_dir().join(format!("sm-render-{}.wav", std::process::id()));
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
}
