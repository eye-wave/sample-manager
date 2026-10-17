//! The glue between `bx_ui` (layout) and `bx_shell` (window, Vulkan, text): an app implements
//! [`App`] (what the screen is, what an event does) and gets the rest: events -> layout -> draw
//! list -> [`Scene`] ([`frame`]), and for a plain one-window app the whole window and render
//! thread ([`run_window`]). Apps depend on this crate alone, it re-exports the others.
pub use bx_shell;
pub use bx_svg;
pub use bx_ui;

use bx_shell::font;
use bx_shell::gpu::{self, Content, Scene, Vert};
use bx_shell::input::{Event, Key};
use bx_shell::window::Window;
use bx_ui::{Cursor, Edit, Layout, Node, Prim, Rects, TextBox, layout, quad_tris};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::time::{Duration, Instant};

/// An icon: vertices (0..1, y down) and triangle indices, what `icons!` makes.
pub type Mesh = (&'static [[f32; 2]], &'static [u32]);

/// `icons!["heart", "star"]` -> `[Mesh; 2]` (lucide names, or paths of SVG files), tessellated at
/// compile time. The index in the list is the id `bx_ui::icon` takes.
#[macro_export]
macro_rules! icons {
    ($($name:literal),* $(,)?) => { [$($crate::bx_svg::icon!($name)),*] };
}

/// A screen: what it shows and how it reacts.
pub trait App {
    /// The meshes of the `bx_ui::icon(id, ..)` nodes of the view, by id.
    const ICONS: &'static [Mesh] = &[];
    /// Draw again 60 times a second even without input (something moves).
    const ANIMATED: bool = false;

    /// The tree to show in a window of `size` px.
    fn view(&self, size: (u32, u32)) -> Node;
    /// Where the nodes with an id were last frame (kept by the app, for hit testing in `event`).
    fn rects_mut(&mut self) -> &mut Rects;

    /// A click, key or wheel turn, oldest first; `mouse` is where the pointer is now (NaN: off
    /// the window).
    fn event(&mut self, _e: Event, _size: (u32, u32), _mouse: [f32; 2]) {}
    /// After the events, before the view: keep state valid for `size`, follow the mouse.
    fn before_view(&mut self, _size: (u32, u32), _mouse: [f32; 2]) {}
    /// After the layout: what the mouse is over now (`layout.rects` is the new set).
    fn after_layout(&mut self, _layout: &Layout, _mouse: [f32; 2]) {}
    /// Extra triangles on top of the page (`scene.flat`), for what a tree cannot say; `t` is the
    /// seconds the app runs.
    fn paint(&self, _scene: &mut Scene, _size: (u32, u32), _t: f32) {}
}

/// One frame of `app`: the `events` happen, then the view is laid out and turned into a [`Scene`].
pub fn frame<A: App>(
    app: &mut A,
    size @ (w, h): (u32, u32),
    mouse: [f32; 2],
    t: f32,
    events: Vec<Event>,
) -> Scene {
    for e in events {
        app.event(e, size, mouse);
    }
    app.before_view(size, mouse);
    let l = layout(
        &app.view(size),
        [0.0, 0.0, w as f32, h as f32],
        &font::width,
        mouse,
    );
    app.after_layout(&l, mouse);
    *app.rects_mut() = l.rects.clone();
    let mut scene = to_scene(l, size, A::ICONS);
    app.paint(&mut scene, size, t);
    scene
}

/// The draw list of a layout as vertices (`icons` is `App::ICONS`).
pub fn to_scene(l: Layout, (w, h): (u32, u32), icons: &[Mesh]) -> Scene {
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
                        let (v, idx) = icons[id];
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

/// Keyboard input for a `TextBox`.
pub trait Typing {
    /// Applies a typed character or key if the box has the focus (Enter and Escape give it up);
    /// whether the event was used.
    fn feed(&mut self, e: &Event) -> bool;
}
impl Typing for TextBox {
    fn feed(&mut self, e: &Event) -> bool {
        if !self.focused {
            return false;
        }
        let edit = match *e {
            Event::Char(c) => Edit::Char(c),
            Event::Key(Key::Enter | Key::Escape) => {
                self.focused = false;
                return true;
            }
            Event::Key(k) => match k {
                Key::Backspace => Edit::Backspace,
                Key::Delete => Edit::Delete,
                Key::CtrlBackspace => Edit::WordBackspace,
                Key::CtrlDelete => Edit::WordDelete,
                Key::CtrlA => Edit::SelectAll,
                Key::Left => Edit::Left,
                Key::Right => Edit::Right,
                Key::Home => Edit::Home,
                _ => Edit::End,
            },
            _ => return false,
        };
        self.edit(edit);
        true
    }
}

/// A fixed size window with `app` in it, until it is closed.
pub fn run_window<A: App + Send>(title: &str, size: (u32, u32), app: A) {
    let win = Window::new(title, size, size, |_, _, _, _| false);
    let stop = AtomicBool::new(false);
    let px = AtomicU64::new(gpu::pack_size(size.0, size.1));
    let (atlas, app, start) = (font::atlas(), Mutex::new(app), Instant::now());
    std::thread::scope(|s| {
        s.spawn(|| {
            let content = Content {
                atlas: &atlas,
                scene: &|w, h, m| {
                    let t = start.elapsed().as_secs_f32();
                    frame(&mut *app.lock().unwrap(), (w, h), m, t, win.events().take())
                },
                mouse: win.mouse(),
                events: win.events(),
            };
            gpu::render(&win, &px, &stop, win.presented(), &content)
        });
        if A::ANIMATED {
            s.spawn(|| {
                while !stop.load(Relaxed) {
                    std::thread::sleep(Duration::from_millis(16));
                    win.events().wake();
                }
            });
        }
        win.run();
        stop.store(true, Relaxed);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use bx_ui::RectExt;

    #[test]
    fn typing_needs_the_focus_and_rects_answer_hit_tests() {
        let mut t = TextBox::new("");
        assert!(!t.feed(&Event::Char('a'))); // not focused: not used
        t.focused = true;
        assert!(
            t.feed(&Event::Char('a')) && t.feed(&Event::Key(Key::Left)) && !t.feed(&Event::Release)
        );
        assert!(t.feed(&Event::Key(Key::Escape)) && !t.focused && t.text == "a");
        let r: Rects = vec![(1, [0.0, 0.0, 10.0, 10.0]), (2, [5.0, 5.0, 10.0, 10.0])].into();
        assert_eq!(r.at([6.0, 6.0]).collect::<Vec<_>>(), [1, 2]);
        assert!(r.hit(2, [14.0, 14.0]) && !r.hit(1, [14.0, 14.0]) && !r.hit(1, [f32::NAN; 2]));
        assert_eq!(r.rect(2).unwrap().hit([6.0, 6.0]), true);
    }
}
