#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::window::App;

mod event;
mod http;
mod window;

fn main() {
    App::build().run();
}
