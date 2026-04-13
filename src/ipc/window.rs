use crate::ipc::IPCBody;
use crate::ipc_commands;

fn close_window(_body: IPCBody) -> Option<std::borrow::Cow<'static, [u8]>> {
    std::process::exit(0);
}

fn minimize_window(body: IPCBody) -> Option<std::borrow::Cow<'static, [u8]>> {
    let w = body.window_handle.as_ref();
    w.set_minimized(true);
    None
}

fn maximize_window(body: IPCBody) -> Option<std::borrow::Cow<'static, [u8]>> {
    let w = body.window_handle.as_ref();
    w.set_maximized(!w.is_maximized());
    None
}

fn drag_window(body: IPCBody) -> Option<std::borrow::Cow<'static, [u8]>> {
    let w = body.window_handle.as_ref();
    w.drag_window().ok();
    None
}

ipc_commands! {
    IPC_WINDOW = [
        close_window,
        minimize_window,
        maximize_window,
        drag_window,
    ]
}
