#![feature(iter_intersperse)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod error;
mod event;
mod http;
mod ipc;
mod plugins;
mod schema;
mod state;
mod window;

pub type AStr = std::sync::Arc<str>;

pub use error::{LogErrorExt, SyncError};

fn main() {
    error::init_logging();

    crate::state::app_paths::create_all_dirs().sure("Failed to create directories");
    crate::window::App::build().run();
}
