//! The app: what is drawn (icons.rs, cube.rs, render.rs) and its two front ends, the standalone window
//! (main.rs) and the CLAP plugin (plugin.rs). Windows, Vulkan and text are the `shell` crate.
mod cube;
pub mod icons;
pub mod render;

#[cfg(feature = "plugin")]
mod plugin;
