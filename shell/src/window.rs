//! The standalone app's window: a thin wrapper over a raw top-level X11 or Win32 window. Every
//! backend exposes the same API:
//!   Window::new     create and show the window, `size` px of client area
//!   Window::presented  an atomic for `render`'s `presented`, so that resizing waits for a fitting frame
//!   Window::run     the event loop: blocks until the user closes the window (call it on the
//!                   thread that created the window)
//!   impl Target     what `gpu::render` draws on (on the render thread)
//!   Drop            destroys the window; the render thread must be gone by then
//! The user cannot resize the window by its edges. Only dragging the handle does, which is
//! handled here (not by the window manager): the width follows the mouse, never below `min.0`, and
//! the height keeps the aspect ratio of `size`. `grip` says whether a point (px from the top
//! left, as `icons::in_grip`) is on the handle of a window of the given size; the mouse cursor
//! changes there.
//! The window is `Sync`, so the render thread can share it with the thread in `run`.
pub type Grip = fn(x: i32, y: i32, w: u32, h: u32) -> bool;

#[cfg(all(unix, not(target_os = "macos")))]
mod linux;
#[cfg(all(unix, not(target_os = "macos")))]
pub use linux::*;

#[cfg(windows)]
mod win;
#[cfg(windows)]
pub use win::*;
