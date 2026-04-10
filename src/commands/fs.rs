use crate::ipc_commands;
use std::borrow::Cow;
use std::sync::Arc;

fn open_folder(_r: &str, _w: &Arc<tao::window::Window>) -> Option<std::borrow::Cow<'static, [u8]>> {
    let folder = tinyfiledialogs::select_folder_dialog("Select folder", "");

    folder.map(|f| Cow::Owned(f.into_bytes()))
}

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
        search_path,
        open_folder,
    ]
}
