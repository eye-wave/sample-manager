//! Dragging a file out of the window into another app (a DAW, a file manager): `drag(path)` is
//! modal, it returns when the mouse button is up and the drop is done (or refused). Call it on
//! the thread that owns the window, from the event that shows the mouse moved with the button
//! held; `input::moved` says when a press has become a drag.
//!   x11   the source side of XDND, a `text/uri-list` with the file; needs the window's display
//!   win   OLE `DoDragDrop` with a `CF_HDROP` data object
#[cfg(all(unix, not(target_os = "macos")))]
mod x11;
#[cfg(all(unix, not(target_os = "macos")))]
pub use x11::drag;

#[cfg(windows)]
mod win;
#[cfg(windows)]
pub use win::drag;
