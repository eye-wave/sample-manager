//! The system's folder picker, no crates: `pick_folder` blocks until the user chose or cancelled,
//! so call it on a thread of its own.
//!   linux  `zenity` or `kdialog` (whichever is installed), polled so `stop` can close it
//!   win    `SHBrowseForFolderW`; it is modal and `stop` cannot close it
#[cfg(all(unix, not(target_os = "macos")))]
mod linux;
#[cfg(all(unix, not(target_os = "macos")))]
pub use linux::pick_folder;

#[cfg(windows)]
mod win;
#[cfg(windows)]
pub use win::pick_folder;
