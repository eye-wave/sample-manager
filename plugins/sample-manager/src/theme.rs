//! Every colour of the GUI, in the schemes of `themes/*.toml`. `t()` is the palette of the scheme the view is being
//! built for (`use_scheme`, set at the start of `view`; per thread, the GUI runs on one).
use bx_app::bx_ui::{Color, rgb};
use std::cell::Cell;
use std::sync::OnceLock;

#[derive(Clone, Copy)]
pub struct Palette {
    // surfaces
    /// Window and table background.
    pub bg: Color,
    /// Sidebar.
    pub panel: Color,
    /// Inputs, buttons, count badges, the selected table row.
    pub field: Color,
    /// Row under the mouse (the file-name bar in the sidebar).
    pub hover: Color,
    /// Table row under the mouse: just a step off `bg`.
    pub row_hover: Color,
    /// Files in the sidebar tree, at rest.
    pub track: Color,
    // strokes
    /// Separators and borders.
    pub line: Color,
    /// The dashed "Add Folder" border.
    pub dash: Color,
    // text and icons
    pub text: Color,
    /// Secondary text, icons and labels.
    pub dim: Color,
    // accent
    pub pink: Color,
    pub pink_text: Color,
    /// Selected entry in the sidebar nav.
    pub nav_sel: Color,
    /// Text on a tag chip.
    pub chip_text: Color,
    /// A tag chip.
    pub tag_bg: Color,
    /// A favourited heart.
    pub love: Color,
    // waveform
    /// The part not played yet.
    pub wave_off: Color,
    /// The played part is a gradient from `wave_a` at the top to `wave_b` at the bottom.
    pub wave_a: Color,
    pub wave_b: Color,
    /// The playhead.
    pub playhead: Color,
}

/// One `0xRRGGBB` or `RRGGBBAA` string of a theme file.
fn hex(s: &str) -> Color {
    let v = u32::from_str_radix(s, 16).unwrap_or_else(|_| panic!("theme colour {s:?} is not hex"));
    match s.len() {
        6 => rgb(v),
        8 => {
            let c = rgb(v >> 8);
            [c[0], c[1], c[2], (v & 0xff) as f32 / 255.0]
        }
        _ => panic!("theme colour {s:?} is not RRGGBB or RRGGBBAA"),
    }
}

pub struct Theme {
    pub name: &'static str,
    pub palette: Palette,
}

/// Compiles `themes/<file>.toml` in and makes a `Theme` of it. The palette takes (the
/// file has more keys than the GUI draws with so far, `radius`, `tag_text_secondary`...):
/// `bg_base` `bg_panel` `bg_surface` `bg_hover` (and `bg_active` for the dashed border), `border`,
/// `accent` `accent_dim` `accent_glow`, `text_primary` (the playhead) `text_secondary` `text_muted`,
/// `tag_bg` `tag_text`, `love` (the heart) and `wave_a` `wave_b` (the waveform).
macro_rules! theme {
    ($id:ident, $path:literal) => {{
        static_toml::static_toml! { static $id = include_toml!($path); }
        let f = &$id;
        Theme {
            name: f.name,
            palette: Palette {
                bg: hex(f.bg_base),
                panel: hex(f.bg_panel),
                field: hex(f.bg_surface),
                hover: hex(f.bg_hover),
                row_hover: hex(f.bg_panel),
                track: hex(f.bg_base),
                line: hex(f.border),
                dash: hex(f.bg_active),
                text: hex(f.text_secondary),
                dim: hex(f.text_muted),
                pink: hex(f.accent),
                pink_text: hex(f.accent_dim),
                nav_sel: hex(f.accent_glow),
                chip_text: hex(f.tag_text),
                tag_bg: hex(f.tag_bg),
                love: hex(f.love),
                wave_off: hex(f.bg_hover),
                wave_a: hex(f.wave_a),
                wave_b: hex(f.wave_b),
                playhead: hex(f.text_primary),
            },
        }
    }};
}

/// The schemes the settings offer, in the order of the chips; the first is the default. To add
/// one: a file in `themes/` and a line here.
pub fn schemes() -> &'static [Theme] {
    static P: OnceLock<Vec<Theme>> = OnceLock::new();
    P.get_or_init(|| {
        vec![
            theme!(VALENTINE, "src/themes/valentine.toml"),
            theme!(VALENTINE_DARK, "src/themes/valentine-dark.toml"),
            theme!(LIGHT, "src/themes/light.toml"),
            theme!(PEACH, "src/themes/peach.toml"),
        ]
    })
}

thread_local!(static CURRENT: Cell<usize> = const { Cell::new(0) });

/// Makes scheme `i` (an index of `schemes()`, clamped) the one `t()` answers with.
pub fn use_scheme(i: usize) {
    CURRENT.set(i.min(schemes().len() - 1));
}

/// The colours of the current scheme.
pub fn t() -> Palette {
    schemes()[CURRENT.get()].palette
}

/// The accent and background of scheme `i`, for the swatches in the settings.
pub fn swatch(i: usize) -> (Color, Color) {
    let p = schemes()[i.min(schemes().len() - 1)].palette;
    (p.pink, p.bg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_file_loads_with_its_name_and_alpha() {
        let names: Vec<_> = schemes().iter().map(|s| s.name).collect();
        assert_eq!(
            names,
            [
                "Valentine",
                "Valentine Dark",
                "Midnight",
                "Phantom",
                "Light",
                "Peach"
            ]
        );
        // RRGGBB is opaque, RRGGBBAA keeps its alpha (the glow is 0x26 of 0xff)
        assert_eq!(schemes()[0].palette.bg, rgb(0x2d2a32));
        assert_eq!(schemes()[0].palette.line[3], 0x1a as f32 / 255.0);
        assert_eq!(schemes()[0].palette.nav_sel[3], 0x26 as f32 / 255.0);
        assert_eq!(schemes()[0].palette.pink[3], 1.0);
        let peach = schemes()[5].palette;
        assert_eq!((peach.love, peach.tag_bg), (rgb(0xff9aa2), rgb(0xffb7b2)));
        assert_ne!(peach.love, peach.pink);
        let p = schemes()[4].palette; // light
        assert_eq!((p.wave_a, p.wave_b), (rgb(0x8ab4ff), rgb(0x2f6fed)));
        use_scheme(99); // clamped to the last one
        assert_eq!(t().bg, schemes()[5].palette.bg);
    }
}
