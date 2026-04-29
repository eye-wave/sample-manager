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
    pub path: Arc<Path>,
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

pub fn clean_up_string(input: &str) -> String {
    input
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

impl FsSample {
    pub fn new(path: Arc<Path>) -> Self {
        let search_str = Arc::from(clean_up_string(&path.to_string_lossy()));
        let tags = tag_string(&path.to_string_lossy());

        Self {
            path,
            search_str,
            tags,
        }
    }

    pub fn score<T: AsRef<str>>(&self, query: &str, tags: &[T], matcher: &SkimMatcherV2) -> i64 {
        if !tags.is_empty() {
            let has_all = tags.iter().all(|t| self.tags.contains(&t.as_ref()));
            if !has_all {
                return i64::MIN;
            }
        }

        matcher
            .fuzzy_match(&self.search_str, query)
            .unwrap_or(i64::MIN)
    }

    pub fn serialize(&self, is_fav: bool) -> String {
        format!(
            r#"{{"path":"{}","tags":{:?},"fav":{is_fav}}}"#,
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
                sample_registry.push(FsSample::new(Arc::from(file_path)));
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
        guard.sample_registry.insert(s.path.clone(), s.clone());
    }

    Ok(())
}
