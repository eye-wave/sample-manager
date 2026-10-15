//! The standalone app's window: a thin wrapper over a raw top-level X11 or Win32 window. Every
//! backend exposes the same API:
//!   EXTS            Vulkan instance extensions to enable
//!   Window::new     create and show the window, `size` px of client area, never smaller than `min`
//!   Window::run     the event loop: blocks until the user closes the window (call it on the
//!                   thread that created the window)
//!   Window::surface create the Vulkan surface (call it on the render thread)
//!   Drop            destroys the window; the render thread must be gone by then
//! `grip` says whether a point (px from the top left, as `icons::in_grip`) is on the resize handle
//! of a window of the given size: dragging there resizes the window, and the mouse cursor shows it.
//! The window is `Sync`, so the render thread can share it with the thread in `run`.
pub type Grip = fn(x: i32, y: i32, w: u32, h: u32) -> bool;

#[cfg(not(any(windows, all(unix, not(target_os = "macos")))))]
compile_error!("window backends exist for Windows and X11 only");
#[cfg(all(unix, not(target_os = "macos")))]
mod linux;
#[cfg(all(unix, not(target_os = "macos")))]
pub use linux::*;

#[cfg(windows)]
mod win;
#[cfg(windows)]
pub use win::*;
