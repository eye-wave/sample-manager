//! The app: what is drawn (icons.rs, view.rs, render.rs) and its two front ends, the standalone window
//! (main.rs) and the CLAP plugin (plugin.rs). Windows, Vulkan and text are the `shell` crate.
pub mod icons;
pub mod render;
mod theme;
mod view;

#[cfg(feature = "plugin")]
mod plugin;
