use crate::commands::IPCCommand;
use crate::ipc_commands;
use std::borrow::Cow;
use std::sync::Arc;

fn search_path(
    path: &str,
    _w: &Arc<tao::window::Window>,
) -> Option<std::borrow::Cow<'static, [u8]>> {
    let files: Vec<_> = std::fs::read_dir(path)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|e| {
            let is_dir = e.path().is_dir() as u8;

            e.path()
                .strip_prefix(path)
                .ok()
                .map(|p| is_dir.to_string() + &p.display().to_string())
        })
        .collect();

    if files.is_empty() {
        None
    } else {
        Some(Cow::Owned(files.join("\n").into_bytes()))
    }
}

ipc_commands! {
    IPC_FS = [
        SearchPath => "search_path" => search_path,
    ]
}
