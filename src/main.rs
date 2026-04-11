#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::{state::AppState, window::App};

mod commands;
mod event;
mod http;
mod state;
mod window;

fn main() {
    AppState::default().create_dirs();
    App::build().run();
}
