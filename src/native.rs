//! The only platform-specific code: a child window inside the host's window, plus a Vulkan
//! surface for it. Every backend exposes the same API:
//!   API / EXTS      CLAP gui api name, Vulkan instance extensions to enable
//!   Native::new     create the child window under the host's parent window, `w` x `h` px
//!   show / hide     map / unmap it
//!   resize          change its size
//!   pointer         where the mouse is, polled (so there is no event handling at all)
//!   surface         create the Vulkan surface (called on the render thread)
//! Everything but `surface` is for the host's main thread. Dropping a `Native` destroys the window.
use ash::{Entry, Instance, khr, vk};
use clack_extensions::gui::{GuiApiType, Window};
use clack_plugin::prelude::PluginError;
use std::ffi::CStr;

/// The mouse, as of the moment of the call.
pub struct Pointer {
    /// Position in the window, px from its top left (can be outside of it).
    pub x: i32,
    pub y: i32,
    /// Position on the screen: stays put while the window moves or resizes under the mouse.
    pub sx: i32,
    pub sy: i32,
    /// Left button held, wherever the mouse is.
    pub down: bool,
}

#[cfg(not(any(windows, all(unix, not(target_os = "macos")))))]
compile_error!("GUI backends exist for Windows and X11 only");
#[cfg(all(unix, not(target_os = "macos")))]
mod linux;
#[cfg(all(unix, not(target_os = "macos")))]
pub use linux::*;

#[cfg(windows)]
mod win;
#[cfg(windows)]
pub use win::*;
