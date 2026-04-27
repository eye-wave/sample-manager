use std::path::PathBuf;
use std::sync::LazyLock;

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

static ICON: LazyLock<Vec<u8>> = LazyLock::new(|| include_bytes!("../../assets/cat.png").to_vec());

fn start_drag_file(body: IPCBody) -> IPCResponse {
    let path = body.req;

    drag::start_drag(
        #[cfg(target_os = "linux")]
        {
            use tao::platform::unix::WindowExtUnix;
            body.window_handle.gtk_window()
        },
        #[cfg(not(target_os = "linux"))]
        &body.window_handle.window(),
        drag::DragItem::Files(vec![PathBuf::from(path.to_string())]),
        drag::Image::Raw(ICON.to_vec()),
        |_, _| {},
        drag::Options::default(),
    )
    .ok();
    ok()
}

ipc_commands! {
    IPC_WINDOW = [
        start_drag_file,
        close_window,
        minimize_window,
        maximize_window,
        drag_window,
    ]
}
