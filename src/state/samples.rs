use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

use crate::state::AppState;

pub struct FsSample {
    pub path: PathBuf,
    search_str: Arc<str>,
}

impl FsSample {
    pub fn new(path: PathBuf) -> Self {
        let stripped = path
            .to_string_lossy()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { ' ' })
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();

        Self {
            path: path.clone(),
            search_str: Arc::from(stripped),
        }
    }

    pub fn score(&self, query: &str, matcher: &SkimMatcherV2) -> i16 {
        let score = matcher
            .fuzzy_match(&self.search_str, query)
            .unwrap_or(i16::MIN as i64);

        score as i16
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
            } else {
                let sample = FsSample::new(file_path);
                sample_registry.push(sample);
            }
        }
    }

    let mut guard = app_state.write().map_err(|_| ())?;
    guard.sample_registry = Arc::from(sample_registry);

    Ok(())
}
