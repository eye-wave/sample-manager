//! The app: what is drawn (icons.rs, view.rs, render.rs) and its two front ends, the standalone window
//! (main.rs) and the CLAP plugin (plugin.rs). Windows, Vulkan and text are the `shell` crate.
pub mod audio;
pub mod icons;
pub mod render;
mod scan;
mod search;
mod settings;
mod sources;
mod state;
mod theme;
mod view;
mod wasm;

#[cfg(feature = "plugin")]
mod plugin;
