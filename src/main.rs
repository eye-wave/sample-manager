#![feature(iter_intersperse)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::{state::app_paths, window::App};

mod audio;
mod event;
mod http;
mod ipc;
mod state;
mod window;

fn main() {
    app_paths::create_all_dirs().ok();
    App::build().run();
}
