//! Every colour of the GUI, in a few schemes. `t()` is the palette of the scheme the view is being
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
    /// Text on a pink tag chip.
    pub chip_text: Color,
    // waveform
    /// The part not played yet.
    pub wave_off: Color,
    /// The playhead.
    pub playhead: Color,
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    [0, 1, 2, 3].map(|i| a[i] + (b[i] - a[i]) * t)
}

/// A scheme from six colours (`0xRRGGBB`); the rest is mixed from them.
fn derive(
    bg: u32,
    panel: u32,
    field: u32,
    text: u32,
    dim: u32,
    accent: u32,
    light: bool,
) -> Palette {
    let c = |x| rgb(x);
    let (bg, panel, field, text, dim, pink) =
        (c(bg), c(panel), c(field), c(text), c(dim), c(accent));
    Palette {
        bg,
        panel,
        field,
        hover: mix(field, text, 0.08),
        row_hover: mix(bg, field, 0.5),
        track: bg,
        line: mix(panel, field, 0.6),
        dash: mix(field, dim, 0.3),
        text,
        dim,
        pink,
        pink_text: mix(pink, bg, 0.05),
        nav_sel: mix(bg, pink, 0.15),
        chip_text: mix(pink, rgb(0), 0.8),
        wave_off: mix(field, dim, 0.15),
        playhead: if light { rgb(0x000000) } else { rgb(0xffffff) },
    }
}

/// The names of the schemes, in the order of `palettes()`.
pub const SCHEMES: [&str; 5] = ["Dusk", "Midnight", "Forest", "Light", "Ember"];

fn palettes() -> &'static [Palette; 5] {
    static P: OnceLock<[Palette; 5]> = OnceLock::new();
    P.get_or_init(|| {
        [
            // the original colours, as they were
            Palette {
                bg: rgb(0x2d2a32),
                panel: rgb(0x37333d),
                field: rgb(0x443f4a),
                hover: rgb(0x4e4856),
                row_hover: rgb(0x36323c),
                track: rgb(0x2d2a32),
                line: rgb(0x3f3b46),
                dash: rgb(0x55505a),
                text: rgb(0xb8b0c5),
                dim: rgb(0x827b8f),
                pink: rgb(0xf4b6c8),
                pink_text: rgb(0xeeaec2),
                nav_sel: rgb(0x4a3942),
                chip_text: rgb(0x3a2a31),
                wave_off: rgb(0x4d4b55),
                playhead: rgb(0xffffff),
            },
            derive(
                0x14161f, 0x1b1e2b, 0x262a3a, 0xc3c9e0, 0x7b83a3, 0x7aa2f7, false,
            ),
            derive(
                0x1c231f, 0x232c27, 0x2d3932, 0xbfd0c4, 0x7e9486, 0x9ece6a, false,
            ),
            derive(
                0xf4f2f7, 0xe9e5ee, 0xdad5e1, 0x3a3544, 0x7d7689, 0xd6336c, true,
            ),
            derive(
                0x241c1a, 0x2e2421, 0x3b2e2a, 0xd9c5bc, 0x9a847a, 0xff9e64, false,
            ),
        ]
    })
}

thread_local!(static CURRENT: Cell<usize> = const { Cell::new(0) });

/// Makes scheme `i` (an index of `SCHEMES`, clamped) the one `t()` answers with.
pub fn use_scheme(i: usize) {
    CURRENT.set(i.min(SCHEMES.len() - 1));
}

/// The colours of the current scheme.
pub fn t() -> Palette {
    palettes()[CURRENT.get()]
}

/// The accent and background of scheme `i`, for the swatches in the settings.
pub fn swatch(i: usize) -> (Color, Color) {
    let p = palettes()[i.min(SCHEMES.len() - 1)];
    (p.pink, p.bg)
}
