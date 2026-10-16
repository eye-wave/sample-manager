//! The example icons (lucide, from `npm install`), tessellated at compile time by
//! `icon_macro::icon!`, which takes any SVG file.
use icon_macro::icon;

/// Vertices (0..1, y down) and triangle indices.
pub type Mesh = (&'static [[f32; 2]], &'static [u32]);

pub const ICONS: [Mesh; 3] = [
    icon!("node_modules/lucide-static/icons/handshake.svg"),
    icon!("node_modules/lucide-static/icons/lightbulb.svg"),
    icon!("node_modules/lucide-static/icons/cpu.svg"),
];

/// The resize handle.
const GRIP_ICON: Mesh = icon!("node_modules/lucide-static/icons/handle-bottom-right.svg");

const SIZE: f32 = 96.0; // px
const GAP: f32 = 24.0;

/// The resize handle is the bottom-right `GRIP` x `GRIP` px square of the window.
pub const GRIP: f32 = 24.0;

/// Whether the point (`x`, `y`) (px, from the top left) is on the resize handle of a `w` x `h` window.
pub fn in_grip(x: i32, y: i32, w: u32, h: u32) -> bool {
    (w as f32 - GRIP..w as f32).contains(&(x as f32))
        && (h as f32 - GRIP..h as f32).contains(&(y as f32))
}

/// Everything drawn, as one NDC triangle list for a `w` x `h` px surface: the icons in a centered
/// row, and the resize handle (a lucide icon) in the bottom-right corner. The length does
/// not depend on `w` and `h`.
pub fn layout(w: u32, h: u32) -> Vec<[f32; 2]> {
    let (w, h) = (w as f32, h as f32);
    let ndc = |x: f32, y: f32| [x / w * 2.0 - 1.0, y / h * 2.0 - 1.0];
    let x0 = (w - (ICONS.len() as f32 * (SIZE + GAP) - GAP)) / 2.0;
    let y0 = (h - SIZE) / 2.0;
    let icons = ICONS.iter().enumerate().flat_map(|(n, (v, idx))| {
        idx.iter().map(move |&i| {
            let [x, y] = v[i as usize];
            (x0 + n as f32 * (SIZE + GAP) + x * SIZE, y0 + y * SIZE)
        })
    });
    let (gx, gy) = (w - GRIP, h - GRIP);
    let grip = GRIP_ICON.1.iter().map(|&i| {
        let [x, y] = GRIP_ICON.0[i as usize];
        (gx + x * GRIP, gy + y * GRIP)
    });
    icons.chain(grip).map(|(x, y)| ndc(x, y)).collect()
}
