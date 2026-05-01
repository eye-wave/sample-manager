#![feature(iter_intersperse)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod event;
mod http;
mod ipc;
mod plugins;
mod state;
mod window;

pub type AStr = std::sync::Arc<str>;
pub type AnyResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() {
    crate::state::app_paths::create_all_dirs().ok();
    crate::window::App::build().run();
}
