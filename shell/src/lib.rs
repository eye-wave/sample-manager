//! The platform layer, no app code: native windows, Vulkan, text. An app brings the icons / scene
//! and calls `gpu::render` on a window.
//!   window  a top-level window (a standalone app)
//!   native  a child window inside somebody else's window (a plugin editor)
//!   gpu     draws a `Scene` (two vertex lists + a texture atlas) on either, see `gpu::Target`
//!   font    MSDF text for the textured half of a `Scene`
#[cfg(not(any(windows, all(unix, not(target_os = "macos")))))]
compile_error!("the platform layer exists for Windows and X11 only");

pub mod font;
pub mod gpu;
pub mod native;
pub mod window;
