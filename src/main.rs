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
    init_logging();

    crate::state::app_paths::create_all_dirs().ok();
    crate::window::App::build().run();
}

fn init_logging() {
    use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(fmt::layer())
        .init();
}
