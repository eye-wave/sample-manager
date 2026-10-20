//! Finds the audio files under the tracked directories.
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::{Arc, Mutex};

const AUDIO: [&str; 7] = ["wav", "mp3", "flac", "ogg", "aif", "aiff", "m4a"];

/// `None` while the scan runs.
pub type Tree = Arc<Mutex<Option<Scan>>>;

pub struct Scan {
    /// The tracked directories, for the sidebar.
    pub roots: Vec<Dir>,
    /// Every audio file found, for the search.
    pub files: Arc<Vec<Sample>>,
}

#[derive(Clone, Default)]
pub struct Sample {
    pub name: String,
    pub path: PathBuf,
    /// The download goes through the plugin's `unpack` (a hit of a search plugin).
    pub unpack: bool,
    /// Frames at the start of the decoded file that are left out (what the plugin says).
    pub skip_frames: usize,
    /// The search plugin (index into `Ui::sources`) this is a hit of; `None` for a local file.
    pub source: Option<usize>,
}

impl Sample {
    /// The file type, like `WAV`.
    pub fn ext(&self) -> String {
        Path::new(&self.name)
            .extension()
            .map_or(String::new(), |x| x.to_string_lossy().to_uppercase())
    }
}

pub struct Dir {
    /// Unique in a scan (depth first order): what a click on the folder names.
    pub id: u32,
    pub name: String,
    /// Where it is on disk (a file in `files` is `path.join(name)`).
    pub path: PathBuf,
    pub dirs: Vec<Dir>,
    /// Audio files directly in here.
    pub files: Vec<String>,
    /// Audio files in here and below.
    pub count: usize,
}

fn walk(path: &Path, name: String, stop: &AtomicBool, all: &mut Vec<Sample>) -> Dir {
    let (mut dirs, mut files) = (vec![], vec![]);
    for e in std::fs::read_dir(path).into_iter().flatten().flatten() {
        let n = e.file_name().to_string_lossy().into_owned();
        // `file_type` does not follow symlinks: no loops, and linked files and folders are skipped
        let Ok(t) = e.file_type() else { continue };
        if stop.load(Relaxed) || n.starts_with('.') {
            continue;
        }
        if t.is_dir() {
            dirs.push(walk(&e.path(), n, stop, all));
        } else if t.is_file()
            && Path::new(&n)
                .extension()
                .is_some_and(|x| AUDIO.iter().any(|a| x.eq_ignore_ascii_case(a)))
        {
            all.push(Sample {
                name: n.clone(),
                path: e.path(),
                ..Default::default()
            });
            files.push(n);
        }
    }
    dirs.sort_by_cached_key(|d| d.name.to_lowercase());
    files.sort_by_cached_key(|f| f.to_lowercase());
    let count = files.len() + dirs.iter().map(|d| d.count).sum::<usize>();
    Dir {
        id: 0,
        name,
        path: path.to_path_buf(),
        dirs,
        files,
        count,
    }
}

fn number(d: &mut Dir, next: &mut u32) {
    d.id = *next;
    *next += 1;
    d.dirs.iter_mut().for_each(|c| number(c, next));
}

/// Scans every root; gives up early (a partial tree) once `stop` is set.
pub fn scan(roots: &[PathBuf], stop: &AtomicBool) -> Scan {
    let (mut next, mut files) = (0, vec![]);
    let roots = roots
        .iter()
        .map(|r| {
            let name = r.file_name().unwrap_or(r.as_os_str()).to_string_lossy();
            let mut d = walk(r, name.into_owned(), stop, &mut files);
            number(&mut d, &mut next);
            d
        })
        .collect();
    Scan {
        roots,
        files: Arc::new(files),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_audio_skips_hidden_and_numbers_uniquely() {
        let root = std::env::temp_dir().join(format!("sm-scan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for f in [
            "a.WAV",
            "b.txt",
            "sub/c.mp3",
            "sub/deep/d.flac",
            ".hid/e.wav",
        ] {
            let p = root.join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, "").unwrap();
        }
        let s = scan(&[root.clone(), root.join("nope")], &AtomicBool::new(false));
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(s.files.len(), 3);
        assert!(
            s.files
                .iter()
                .any(|f| f.name == "d.flac" && f.ext() == "FLAC")
        );
        let t = s.roots;
        assert_eq!(
            (t[0].count, &t[0].files[..], t[0].dirs.len()),
            (3, &["a.WAV".to_string()][..], 1)
        );
        assert_eq!(
            (t[0].dirs[0].id, t[0].dirs[0].dirs[0].id, t[1].id),
            (1, 2, 3)
        );
        assert_eq!(t[1].count, 0);
    }
}
