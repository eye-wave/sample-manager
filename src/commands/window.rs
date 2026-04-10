use crate::ipc_commands;
use std::sync::Arc;

fn close_window(
    _req: &str,
    _w: &Arc<tao::window::Window>,
) -> Option<std::borrow::Cow<'static, [u8]>> {
    std::process::exit(0);
}

fn minimize_window(
    _req: &str,
    w: &Arc<tao::window::Window>,
) -> Option<std::borrow::Cow<'static, [u8]>> {
    w.set_minimized(true);
    None
}

fn maximize_window(
    _req: &str,
    w: &Arc<tao::window::Window>,
) -> Option<std::borrow::Cow<'static, [u8]>> {
    w.set_maximized(!w.is_maximized());
    None
}

fn drag_window(
    _req: &str,
    w: &Arc<tao::window::Window>,
) -> Option<std::borrow::Cow<'static, [u8]>> {
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
