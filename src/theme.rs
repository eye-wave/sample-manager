//! Every colour of the GUI. Tweak them here, `0xRRGGBB`.
use bx_ui::{Color, rgb};

// surfaces
/// Window and table background.
pub const BG: Color = rgb(0x2d2a32);
/// Sidebar.
pub const PANEL: Color = rgb(0x37333d);
/// Inputs, buttons, count badges, the selected table row.
pub const FIELD: Color = rgb(0x443f4a);
/// Row under the mouse (the file-name bar in the sidebar).
pub const HOVER: Color = rgb(0x4e4856);
/// Table row under the mouse: just a step off `BG`.
pub const ROW_HOVER: Color = rgb(0x36323c);
/// Files in the sidebar tree, at rest.
pub const TRACK: Color = rgb(0x2d2a32);

// strokes
/// Separators and borders.
pub const LINE: Color = rgb(0x3f3b46);
/// The dashed "Add Folder" border.
pub const DASH: Color = rgb(0x55505a);

// text and icons
pub const TEXT: Color = rgb(0xb8b0c5);
/// Secondary text, icons and labels.
pub const DIM: Color = rgb(0x827b8f);

// accent
pub const PINK: Color = rgb(0xf4b6c8);
pub const PINK_TEXT: Color = rgb(0xeeaec2);
/// Selected entry in the sidebar nav.
pub const NAV_SEL: Color = rgb(0x4a3942);
/// Text on a pink tag chip.
pub const CHIP_TEXT: Color = rgb(0x3a2a31);

// waveform
/// The part not played yet.
pub const WAVE_OFF: Color = rgb(0x4d4b55);
/// The playhead.
pub const PLAYHEAD: Color = rgb(0xffffff);
