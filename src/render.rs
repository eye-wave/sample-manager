//! The GUI: what is drawn (icons, resize handle, text) for a given window size. No Vulkan calls
//! here, gpu.rs draws what [`scene`] returns. Shared by the standalone app and the plugin.
use crate::gpu::{self, Atlas, Scene};
use crate::{font, icons::layout};
use ash::{Entry, Instance, vk};
use std::ffi::c_char;
use std::sync::atomic::{AtomicBool, AtomicU64};

pub use crate::gpu::pack_size;

/// Initial window size in px.
pub const W: u32 = 400;
pub const H: u32 = 300;
/// Smallest allowed window size: the icon row (336 px) plus some margin.
pub const MIN_W: u32 = 360;
pub const MIN_H: u32 = 160;

fn scene(w: u32, h: u32) -> Scene {
    let mut textured = vec![];
    // baselines picked so the text clears the icon row at the smallest window size
    font::text(&mut textured, "sample-manager", (16.0, 26.0), 18.0, (w, h));
    font::text(
        &mut textured,
        "msdf text, no layout yet",
        (16.0, h as f32 - 12.0),
        12.0,
        (w, h),
    );
    Scene {
        flat: layout(w, h),
        textured,
    }
}

/// Draws the GUI on the surface that `surface` returns until `stop` is set: [`gpu::render`] with
/// this module's scene and the font atlas, see there for the arguments and the safety contract.
///
/// # Safety
///
/// As [`gpu::render`].
pub unsafe fn render(
    exts: &[*const c_char],
    surface: impl FnOnce(&Entry, &Instance) -> vk::SurfaceKHR,
    size: &AtomicU64,
    stop: &AtomicBool,
) {
    let rgba = font::atlas_rgba();
    let atlas = Atlas {
        w: font::ATLAS_W,
        h: font::ATLAS_H,
        rgba: &rgba,
    };
    unsafe { gpu::render(exts, surface, size, stop, &atlas, scene) }
}
