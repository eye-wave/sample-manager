use std::sync::{Arc, RwLock};

use crate::{ipc_commands, state::AppState};

fn log(
    req: &str,
    _w: &Arc<tao::window::Window>,
    _s: Arc<RwLock<AppState>>,
) -> Option<std::borrow::Cow<'static, [u8]>> {
    let mode = req.as_bytes().first().map(|&b| b as char)?;
    let message = &req[1..];

    const RESET: &str = "\x1b[0m";

    let ansi = match mode {
        'L' => Some("\x1b[37m"),
        'W' => Some("\x1b[33m"),
        'E' => Some("\x1b[31m"),
        _ => None,
    }
    .unwrap_or("");

    println!("[WEB] {ansi}{message}{RESET}");

    None
}

ipc_commands! {
    IPC_LOGGER = [
        log
    ]
}
