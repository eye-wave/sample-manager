//! Two front ends over one renderer: the `standalone` feature builds the app (main.rs, window/),
//! the `plugin` feature builds the CLAP plugin (plugin.rs, native.rs). The renderer is split into
//! the GUI (render.rs, font.rs, icons.rs: what is drawn) and raw Vulkan (gpu.rs: how).
mod font;
mod gpu;
pub mod icons;
pub mod render;

#[cfg(feature = "standalone")]
pub mod window;

#[cfg(feature = "plugin")]
mod native;
#[cfg(feature = "plugin")]
mod plugin;
