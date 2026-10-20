//! The icons (lucide, from `npm install`), tessellated at compile time by `icon!`, which takes any
//! SVG file. The ids are indices into [`ICONS`]; the layout engine only knows the number.
use bx_app::{Mesh, icons};

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
pub const PLAY: usize = 12;
pub const X: usize = 13;
/// The heart with its inside: a favorite.
pub const HEART_FILL: usize = 14;
pub const GLOBE: usize = 15;

pub const ICONS: [Mesh; 16] = icons![
    "search",
    "settings",
    "house",
    "heart",
    "music",
    "folder",
    "chevron-right",
    "plus",
    "pause",
    "handle-bottom-right",
    "folder-open",
    "chevron-down",
    "play",
    "x",
    "heart:fill",
    "globe",
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
    fn the_filled_heart_has_more_triangles_than_the_outline() {
        assert!(super::ICONS[super::HEART_FILL].1.len() > super::ICONS[super::HEART].1.len());
    }

    #[test]
    fn grip_is_the_bottom_right_corner() {
        assert!(super::in_grip(399, 299, 400, 300) && super::in_grip(380, 280, 400, 300));
        assert!(!super::in_grip(200, 150, 400, 300) && !super::in_grip(399, 200, 400, 300));
        assert!(!super::in_grip(400, 300, 400, 300));
    }
}
