//! A child window inside a window that somebody else owns (a plugin host's editor parent), plus
//! a Vulkan surface for it. Every backend exposes the same API:
//!   Native::new     create the child window under `parent` (the native handle: an X11 window id
//!                   or an HWND, as usize), `w` x `h` px
//!   show / hide     map / unmap it
//!   resize          change its size
//!   pointer         where the mouse is, polled (so there is no event handling at all)
//!   impl Target     what `gpu::render` draws on (on the render thread)
//! Everything but the surface is for the thread that created the window. Dropping a `Native`
//! destroys the window.
use crate::gpu::Target;
use ash::{Entry, Instance, khr, vk};

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

#[cfg(all(unix, not(target_os = "macos")))]
mod linux;
#[cfg(all(unix, not(target_os = "macos")))]
pub use linux::*;

#[cfg(windows)]
mod win;
#[cfg(windows)]
pub use win::*;
