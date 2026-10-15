//! The example icons, tessellated at compile time by `lucide_macro::icon!` (needs `npm install`).
use lucide_macro::icon;

/// Vertices (0..1, y down) and triangle indices.
pub type Mesh = (&'static [[f32; 2]], &'static [u32]);

pub const ICONS: [Mesh; 3] = [
    icon!("node_modules/lucide-static/icons/handshake.svg"),
    icon!("node_modules/lucide-static/icons/lightbulb.svg"),
    icon!("node_modules/lucide-static/icons/cpu.svg"),
];

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
/// row, and the resize handle (a triangle of six dots) in the bottom-right corner. The length does
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
    // 3 px dots, 6 px apart, 5 px from the corner; (i, j) counts from the right / from the bottom
    let dots = [(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (0, 2)]
        .into_iter()
        .flat_map(|(i, j): (u8, u8)| {
            let (r, b) = (w - 5.0 - i as f32 * 6.0, h - 5.0 - j as f32 * 6.0);
            let (l, t) = (r - 3.0, b - 3.0);
            [(l, t), (r, t), (l, b), (r, t), (r, b), (l, b)]
        });
    icons.chain(dots).map(|(x, y)| ndc(x, y)).collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_icon_is_a_nonempty_mesh() {
        for (v, i) in super::ICONS {
            assert!(!v.is_empty() && i.len() % 3 == 0 && i.iter().all(|&i| (i as usize) < v.len()));
        }
    }

    #[test]
    fn layout_fits_every_allowed_size_and_has_a_fixed_length() {
        let n = super::layout(400, 300).len();
        for (w, h) in [
            (400, 300),
            (crate::render::MIN_W, crate::render::MIN_H),
            (1920, 1080),
        ] {
            let t = super::layout(w, h);
            assert_eq!(t.len(), n);
            assert!(
                t.iter().flatten().all(|c| (-1.0..=1.0).contains(c)),
                "{w}x{h}"
            );
        }
    }

    #[test]
    fn grip_is_the_bottom_right_corner() {
        assert!(super::in_grip(399, 299, 400, 300) && super::in_grip(380, 280, 400, 300));
        assert!(!super::in_grip(200, 150, 400, 300) && !super::in_grip(399, 200, 400, 300));
        assert!(!super::in_grip(400, 300, 400, 300));
    }
}
