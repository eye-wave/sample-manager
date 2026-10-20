//! The pickers from a helper program: `zenity` (GTK, most desktops) or `kdialog` (KDE).
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::time::Duration;

/// What the helper printed: the chosen folder on one line, nothing if it was cancelled.
fn parse(out: &str) -> Option<PathBuf> {
    let line = out.lines().next()?.trim_end_matches('\r');
    (!line.is_empty()).then(|| PathBuf::from(line))
}

/// Asks for a folder; `None` if cancelled, if `stop` was set (the dialog is closed then) or if no
/// helper is installed.
pub fn pick_folder(stop: &AtomicBool) -> Option<PathBuf> {
    pick(
        [
            (
                "zenity",
                &["--file-selection", "--directory", "--title=Add folder"],
            ),
            (
                "kdialog",
                &["--getexistingdirectory", ".", "--title", "Add folder"],
            ),
        ],
        stop,
    )
}

/// The same for one `.wasm` file.
pub fn pick_wasm(stop: &AtomicBool) -> Option<PathBuf> {
    pick(
        [
            (
                "zenity",
                &[
                    "--file-selection",
                    "--title=Add plugin",
                    "--file-filter=WebAssembly | *.wasm",
                ],
            ),
            (
                "kdialog",
                &[
                    "--getopenfilename",
                    ".",
                    "*.wasm|WebAssembly",
                    "--title",
                    "Add plugin",
                ],
            ),
        ],
        stop,
    )
}

fn pick(helpers: [(&str, &[&str]); 2], stop: &AtomicBool) -> Option<PathBuf> {
    for (cmd, args) in helpers {
        let Ok(mut child) = Command::new(cmd)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        else {
            continue; // not installed: try the next one
        };
        let mut out = child.stdout.take()?;
        let status = loop {
            if stop.load(Relaxed) {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            match child.try_wait() {
                Ok(Some(s)) => break s,
                Ok(None) => std::thread::sleep(Duration::from_millis(50)),
                Err(_) => return None,
            }
        };
        let mut text = String::new();
        // a path is far smaller than the pipe buffer, so reading after the exit loses nothing
        let _ = out.read_to_string(&mut text);
        return status.success().then(|| parse(&text)).flatten();
    }
    eprintln!("no file picker: install zenity or kdialog");
    None
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_helper_output_is_one_path_or_nothing() {
        assert_eq!(
            super::parse("/home/a b/Music\n"),
            Some("/home/a b/Music".into())
        );
        assert_eq!(super::parse(""), None);
        assert_eq!(super::parse("\n"), None);
    }
}
