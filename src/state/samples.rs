use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

use crate::state::AppState;

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

pub struct FsSample {
    pub path: PathBuf,
    search_str: Arc<str>,
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

        Self {
            path,
            search_str: Arc::from(search_str),
        }
    }

    pub fn score(&self, query: &str, matcher: &SkimMatcherV2) -> i64 {
        matcher
            .fuzzy_match(&self.search_str, query)
            .unwrap_or(i64::MIN)
    }
}

pub fn process_directories(
    dirs: HashSet<PathBuf>,
    app_state: Arc<RwLock<AppState>>,
) -> Result<(), ()> {
    let mut sample_registry = Vec::<FsSample>::new();
    let mut stack: Vec<PathBuf> = dirs.into_iter().collect();

    while let Some(current_dir) = stack.pop() {
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

    let mut guard = app_state.write().map_err(|_| ())?;
    guard.sample_registry = Arc::from(sample_registry);

    Ok(())
}
