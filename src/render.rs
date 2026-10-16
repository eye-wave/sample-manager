//! The GUI: what is drawn (icons, resize handle, text, the cube) for a given window size. No Vulkan or
//! windowing here, `shell::gpu` draws what [`scene`] returns (and spins the cube, see cube.rs). Shared by the standalone app and
//! the plugin.
use crate::cube;
use crate::icons::layout;
use bx_shell::font;
use bx_shell::gpu::{self, Content, Scene, Target};
use std::sync::atomic::{AtomicBool, AtomicU64};

pub use bx_shell::gpu::pack_size;

/// Initial window size in px.
pub const W: u32 = 400;
pub const H: u32 = 300;
/// Smallest allowed window size, the same shape as the initial one.
pub const MIN_W: u32 = 360;
pub const MIN_H: u32 = MIN_W * H / W;

/// The window keeps the aspect ratio of `W` x `H`: the height that goes with width `w`.
pub const fn height_for(w: u32) -> u32 {
    w * H / W
}

/// What is drawn in a `w` x `h` px window.
pub fn scene(w: u32, h: u32) -> Scene {
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
        mesh: cube::mesh(),
    }
}

/// Draws the GUI on `target` until `stop` is set: [`gpu::render`] with this module's scene and
/// the font atlas, see there for the arguments. Run it on a thread of its own.
pub fn run(target: &impl Target, size: &AtomicU64, stop: &AtomicBool, presented: &AtomicU64) {
    let atlas = font::atlas();
    let content = Content {
        atlas: &atlas,
        scene: &scene,
        mesh_shader: cube::SHADER,
        uniform: &cube::mvp,
    };
    gpu::render(target, size, stop, presented, &content)
}
