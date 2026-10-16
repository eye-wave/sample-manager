//! The icons (lucide, from `npm install`), tessellated at compile time by `icon!`, which takes any
//! SVG file. The ids are indices into [`ICONS`]; the layout engine only knows the number.
use bx_svg::icon;

/// Vertices (0..1, y down) and triangle indices.
pub type Mesh = (&'static [[f32; 2]], &'static [u32]);

pub const SEARCH: usize = 0;
pub const SETTINGS: usize = 1;
pub const HOUSE: usize = 2;
pub const HEART: usize = 3;
pub const MUSIC: usize = 4;
pub const FOLDER: usize = 5;
pub const CHEVRON: usize = 6;
pub const PLUS: usize = 7;
pub const PAUSE: usize = 8;
pub const GRIP_ICON: usize = 9;
pub const FOLDER_OPEN: usize = 10;
pub const CHEVRON_DOWN: usize = 11;

pub const ICONS: [Mesh; 12] = [
    icon!("node_modules/lucide-static/icons/search.svg"),
    icon!("node_modules/lucide-static/icons/settings.svg"),
    icon!("node_modules/lucide-static/icons/house.svg"),
    icon!("node_modules/lucide-static/icons/heart.svg"),
    icon!("node_modules/lucide-static/icons/music.svg"),
    icon!("node_modules/lucide-static/icons/folder.svg"),
    icon!("node_modules/lucide-static/icons/chevron-right.svg"),
    icon!("node_modules/lucide-static/icons/plus.svg"),
    icon!("node_modules/lucide-static/icons/pause.svg"),
    icon!("node_modules/lucide-static/icons/move-diagonal-2.svg"),
    icon!("node_modules/lucide-static/icons/folder-open.svg"),
    icon!("node_modules/lucide-static/icons/chevron-down.svg"),
];

/// The resize handle is the bottom-right `GRIP` x `GRIP` px square of the window (the view draws
/// its icon there).
pub const GRIP: f32 = 24.0;

/// Whether the point (`x`, `y`) (px, from the top left) is on the resize handle of a `w` x `h` window.
pub fn in_grip(x: i32, y: i32, w: u32, h: u32) -> bool {
    (w as f32 - GRIP..w as f32).contains(&(x as f32))
        && (h as f32 - GRIP..h as f32).contains(&(y as f32))
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
    fn grip_is_the_bottom_right_corner() {
        assert!(super::in_grip(399, 299, 400, 300) && super::in_grip(380, 280, 400, 300));
        assert!(!super::in_grip(200, 150, 400, 300) && !super::in_grip(399, 200, 400, 300));
        assert!(!super::in_grip(400, 300, 400, 300));
    }
}
