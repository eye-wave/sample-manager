use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock};

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

use crate::ipc::IPCMessage;
use crate::state::AppState;

mod tagger;

pub use tagger::tag_string;

pub const SAMPLE_EXTENSIONS: &[&str] = &[
    "aac", "aiff", "caf", "flac", "mid", "midi", "mp2", "mp3", "mp4", "mpeg", "ogg", "opus", "wav",
    "wv",
];

fn is_sample_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| SAMPLE_EXTENSIONS.binary_search(&ext).is_ok())
        .unwrap_or(false)
}

#[derive(Debug, Clone)]
pub struct FsSample {
    pub path: PathBuf,
    pub tags: Vec<&'static str>,

    search_str: Arc<str>,
}

impl PartialEq for FsSample {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Eq for FsSample {}

impl std::hash::Hash for FsSample {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.path.hash(state);
    }
}

impl FsSample {
    pub fn new(path: PathBuf) -> Self {
        let search_str = path
            .to_string_lossy()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { ' ' })
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();

        let tags = tag_string(&path.to_string_lossy());

        Self {
            path,
            search_str: Arc::from(search_str),
            tags,
        }
    }

    pub fn score(&self, query: &str, tags: &[&str], matcher: &SkimMatcherV2) -> i64 {
        let has_all = tags.iter().all(|t| self.tags.contains(t));

        if !has_all {
            return 0;
        }

        matcher
            .fuzzy_match(&self.search_str, query)
            .unwrap_or(i64::MIN)
    }

    pub fn serialize(&self) -> String {
        format!(
            r#"{{"path":"{}","tags":{:?}}}"#,
            self.path.to_string_lossy().replace("\\", "\\\\"),
            self.tags
        )
    }
}

pub fn process_directories<'a>(
    dirs: impl Iterator<Item = &'a PathBuf>,
    app_state: Arc<RwLock<AppState>>,
    sender: Sender<IPCMessage>,
) -> Result<(), ()> {
    use std::time::{Duration, Instant};

    let mut sample_registry = Vec::<FsSample>::new();
    let mut stack: Vec<PathBuf> = dirs.into_iter().cloned().collect();

    let mut time = Instant::now();

    while let Some(current_dir) = stack.pop() {
        if time.elapsed() >= Duration::from_millis(398) {
            sender
                .send(IPCMessage {
                    id: "s_tick",
                    payload: sample_registry.len().to_string(),
                })
                .ok();

            time = Instant::now();
        }

        let entries = match std::fs::read_dir(&current_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let file_path = entry.path();

            if file_path.is_dir() {
                stack.push(file_path);
            } else if is_sample_file(&file_path) {
                sample_registry.push(FsSample::new(file_path));
            }
        }
    }

    sender
        .send(IPCMessage {
            id: "s_tick",
            payload: sample_registry.len().to_string(),
        })
        .ok();

    println!("SCAN DONE!");

    let mut guard = app_state.write().map_err(|_| ())?;
    for s in sample_registry.iter() {
        guard.sample_registry.insert(s.clone());
    }

    Ok(())
}
