use crate::ipc::{IPCBody, IPCError, IPCResponse, ok};
use crate::ipc_commands;

fn log(body: IPCBody) -> IPCResponse {
    use std::io::{self, Write};

    let mode = body
        .req
        .as_bytes()
        .first()
        .map(|&b| b as char)
        .ok_or(IPCError::from("Missing logger mode"))?;

    let message = &body.req[1..];

    const GRAY: &str = "\x1b[90m";
    const RESET: &str = "\x1b[0m";

    let ansi = match mode {
        'L' => "\x1b[37m",
        'W' => "\x1b[33m",
        'E' => "\x1b[31m",
        _ => "",
    };

    let now = chrono::Local::now().format("%H:%M:%S");
    let formatted_message = message.replace(RESET, ansi);

    let mut stdout = io::stdout().lock();
    let _ = writeln!(
        stdout,
        "{GRAY}{now}\x1b[0m [🌐WEB] {ansi}{formatted_message}\x1b[0m"
    );

    ok()
}

ipc_commands! {
    IPC_LOGGER = [
        log
    ]
}
