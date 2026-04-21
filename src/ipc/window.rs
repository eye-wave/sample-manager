use crate::ipc::{IPCBody, IPCResponse, ok};
use crate::ipc_commands;

fn close_window(_body: IPCBody) -> IPCResponse {
    std::process::exit(0);
}

fn minimize_window(body: IPCBody) -> IPCResponse {
    let w = body.window_handle.as_ref();
    w.set_minimized(true);

    ok()
}

fn maximize_window(body: IPCBody) -> IPCResponse {
    let w = body.window_handle.as_ref();
    w.set_maximized(!w.is_maximized());

    ok()
}

fn drag_window(body: IPCBody) -> IPCResponse {
    let w = body.window_handle.as_ref();
    w.drag_window()?;

    ok()
}

ipc_commands! {
    IPC_WINDOW = [
        close_window,
        minimize_window,
        maximize_window,
        drag_window,
    ]
}
