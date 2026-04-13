use std::path::Path;

use crate::ipc::{IPCBody, IPCResponse};
use crate::ipc_commands;

fn open_folder(_body: IPCBody) -> Option<std::borrow::Cow<'static, [u8]>> {
    let folder = tinyfiledialogs::select_folder_dialog("Select folder", "");
    folder.finish()
}

fn get_path_type(path: &Path) -> u8 {
    if path.is_dir() {
        return 0;
    }

    // Lists must remain sorted for binary_search to work correctly
    const AUDIO_SAMPLE_DATA: &[&str] = &[
        "aac", "aiff", "caf", "flac", "mp2", "mp3", "mp4", "mpeg", "ogg", "opus", "wav", "wv",
    ];
    const AUDIO_MIDI_DATA: &[&str] = &["mid", "midi"];
    const PLUGIN_PRESETS: &[&str] = &["fxb", "fxp", "vital"];

    let ext = match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => ext,
        None => return 90,
    };

    let contains = |list: &[&str]| list.binary_search(&ext).is_ok();

    if contains(AUDIO_SAMPLE_DATA) {
        return 1;
    }
    if contains(AUDIO_MIDI_DATA) {
        return 2;
    }
    if contains(PLUGIN_PRESETS) {
        return 4;
    }

    90
}

fn read_dir(body: IPCBody) -> Option<std::borrow::Cow<'static, [u8]>> {
    const BYTE_OFFSET: u8 = 32;

    let path = body.req.as_ref();
    let mut files: Vec<String> = std::fs::read_dir(path)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|e| {
            let item_type = get_path_type(&e.path()) + BYTE_OFFSET;
            let prefix = unsafe { String::from_utf8_unchecked(vec![item_type]) };

            let relative = e.path().strip_prefix(path).ok()?.display().to_string();
            Some(prefix + &relative)
        })
        .collect();

    files.sort();

    if files.is_empty() {
        None
    } else {
        files.join("\n").finish()
    }
}

ipc_commands! {
    IPC_FS = [
        read_dir,
        open_folder,
    ]
}
