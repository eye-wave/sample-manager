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

fn main() {
    crate::state::app_paths::create_all_dirs().ok();
    crate::window::App::build().run();
}
