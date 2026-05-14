#![feature(iter_intersperse)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod error;
mod event;
mod http;
mod ipc;
mod logger;
mod plugins;
mod schema;
mod state;
mod window;

pub type AStr = std::sync::Arc<str>;

pub use error::{LogErrorExt, SyncError};

fn main() {
    crate::state::app_paths::create_all_dirs().sure("Failed to create directories");
    let win = crate::window::App::build();

    logger::init_logging(win.webview_sender.clone());

    win.run();
}
