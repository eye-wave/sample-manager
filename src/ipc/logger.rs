use crate::ipc::IPCBody;
use crate::ipc_commands;

fn log(body: IPCBody) -> Option<std::borrow::Cow<'static, [u8]>> {
    let mode = body.req.as_bytes().first().map(|&b| b as char)?;
    let message = &body.req[1..];

    const RESET: &str = "\x1b[0m";

    let ansi = match mode {
        'L' => "\x1b[37m",
        'W' => "\x1b[33m",
        'E' => "\x1b[31m",
        _ => "",
    };

    println!("[🌐WEB] {ansi}{message}{RESET}");

    None
}

ipc_commands! {
    IPC_LOGGER = [
        log
    ]
}
