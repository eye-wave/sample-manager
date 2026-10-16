//! What survives a restart: the tracked directories, one path per line in
//! `<config dir>/sample-manager/dirs.txt`. Standalone and plugin share it.
use std::path::PathBuf;

fn file() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join("sample-manager").join("dirs.txt"))
}

/// The tracked directories; the first run (no file yet) tracks `$HOME/Music` and saves that.
// ponytail: a path with a newline in it does not round-trip; switch to postcard/serde if state grows
pub fn tracked_dirs() -> Vec<PathBuf> {
    let Some(f) = file() else { return vec![] };
    if let Ok(s) = std::fs::read_to_string(&f) {
        return s
            .lines()
            .filter(|l| !l.is_empty())
            .map(PathBuf::from)
            .collect();
    }
    let list: Vec<PathBuf> = dirs::home_dir()
        .map(|h| h.join("Music"))
        .into_iter()
        .collect();
    save(&f, &list);
    list
}

fn save(f: &std::path::Path, list: &[PathBuf]) {
    let text: Vec<_> = list.iter().map(|p| p.to_string_lossy()).collect();
    // not being able to save is not worth stopping for: the next start tracks the default again
    let _ = f.parent().map(std::fs::create_dir_all);
    let _ = std::fs::write(f, text.join("\n"));
}
