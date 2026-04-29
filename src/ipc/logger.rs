use crate::ipc::{IPCBody, IPCError, IPCResponse, ok};
use crate::ipc_commands;

fn log(body: IPCBody) -> IPCResponse {
    let mode = body
        .req
        .as_bytes()
        .first()
        .map(|&b| b as char)
        .ok_or(IPCError::from("Missing logger mode"))?;

    let message = &body.req[1..];

    const RESET: &str = "\x1b[0m";

    let ansi = match mode {
        'L' => "\x1b[37m",
        'W' => "\x1b[33m",
        'E' => "\x1b[31m",
        _ => "",
    };

    println!("[🌐WEB] {ansi}{}{RESET}", message.replace(RESET, ansi));
    ok()
}

ipc_commands! {
    IPC_LOGGER = [
        log
    ]
}
