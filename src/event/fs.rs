use std::borrow::Cow;

use super::IPCCommand;

pub struct SearchPath;

impl IPCCommand for SearchPath {
    fn name(&self) -> &'static str {
        "search_path"
    }

    fn respond(&self, path: &str) -> Option<Cow<'static, [u8]>> {
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
}
