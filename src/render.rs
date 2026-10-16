//! The GUI: what is drawn for a given window size. No Vulkan or windowing here, `shell::gpu` draws
//! what [`scene`] returns. Shared by the standalone app and the plugin. The tree is view.rs, the
//! layout is `bx_ui`; this file only turns the draw list into vertices.
use crate::icons::ICONS;
use crate::view::view;
use bx_shell::font;
use bx_shell::gpu::{self, Content, Scene, Target, Vert};
use bx_ui::{Cursor, Prim, layout, quad_tris};
use std::sync::atomic::{AtomicBool, AtomicU64};

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

/// What is drawn in a `w` x `h` px window with the mouse at `mouse` (px, NaN if it is elsewhere).
pub fn scene(w: u32, h: u32, mouse: [f32; 2]) -> Scene {
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
    let l = layout(&view(w), [0.0, 0.0, fw, fh], &font::width, mouse);
    let ((flat, textured), (over_flat, over_textured)) = (layer(l.base), layer(l.over));
    Scene {
        flat,
        textured,
        over_flat,
        over_textured,
        pointer: l.cursor == Cursor::Pointer,
    }
}

/// Draws the GUI on `target` until `stop` is set: [`gpu::render`] with this module's scene and
/// the font atlas, see there for the arguments (`mouse` is `pack_mouse`d, `NO_MOUSE` when off the
/// window). Run it on a thread of its own.
pub fn run(
    target: &impl Target,
    size: &AtomicU64,
    stop: &AtomicBool,
    presented: &AtomicU64,
    mouse: &AtomicU64,
) {
    let atlas = font::atlas();
    let content = Content {
        atlas: &atlas,
        scene: &scene,
        mouse,
    };
    gpu::render(target, size, stop, presented, &content)
}
