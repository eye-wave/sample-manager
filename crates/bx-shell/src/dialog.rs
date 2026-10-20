//! The system's file pickers, no crates: `pick_folder` and `pick_wasm` (one `.wasm` file) block until
//! the user chose or cancelled, so call them on a thread of their own.
//!   linux  `zenity` or `kdialog` (whichever is installed), polled so `stop` can close it
//!   win    `SHBrowseForFolderW` / `GetOpenFileNameW`; they are modal and `stop` cannot close them
#[cfg(all(unix, not(target_os = "macos")))]
mod linux;
#[cfg(all(unix, not(target_os = "macos")))]
pub use linux::{pick_folder, pick_wasm};

#[cfg(windows)]
mod win;
#[cfg(windows)]
pub use win::{pick_folder, pick_wasm};
