use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use plugin_wire::sample::SampleMetadata;

use crate::ipc::IPCSenderUI;
use crate::state::AppState;
use crate::{AStr, SyncError};

mod data;
mod search;
mod tagger;
mod waveform;

pub mod utils;

pub use data::{PluginSample, SampleDataError, SampleEntry, SampleSerialize, SampleSource};
pub use search::{SearchRequest, filter_samples, search_local};
pub use tagger::tag_string;
pub use waveform::{WaveformData, draw_audio_and_save};

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
    search_str: AStr,
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
    pub fn new(path: impl AsRef<Path>) -> Self {
        let path_str = path.as_ref().to_string_lossy();
        let search_str = Arc::from(clean_up_string(&path_str));
        let tags = tag_string(&path_str);
        Self {
            path: PathBuf::from(path.as_ref()),
            search_str,
            tags,
        }
    }
}

impl SampleEntry for FsSample {
    fn hash_key(&self) -> Result<&str, SampleDataError> {
        self.path
            .to_str()
            .ok_or(SampleDataError::PathConversionError)
    }

    fn score(&self, query: &str, tags: &[&str], matcher: &SkimMatcherV2) -> i64 {
        if !tags.is_empty() {
            let has_all = tags.iter().all(|t| self.tags.contains(t));
            if !has_all {
                return i64::MIN;
            }
        }
        matcher
            .fuzzy_match(&self.search_str, query)
            .unwrap_or(i64::MIN)
    }

    fn to_serialize(&self) -> Result<SampleSerialize, SampleDataError> {
        Ok(SampleSerialize {
            name: self
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            source: SampleSource::Native {
                path: self.path.to_string_lossy().into_owned(),
            },
            meta: SampleMetadata {
                bpm: None,
                description: None,
                sample_type: plugin_wire::SampleType::OneShot,
                tags: self.tags.iter().map(|s| Arc::from(*s)).collect(),
            },
        })
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

pub enum ScanMerge {
    ReplaceAll,
    ReplaceUnder(Vec<PathBuf>),
}

pub fn process_directories<'a>(
    dirs: impl Iterator<Item = &'a PathBuf>,
    merge: ScanMerge,
    app_state: Arc<RwLock<AppState>>,
    sender: IPCSenderUI,
) -> Result<(), SyncError> {
    use std::time::{Duration, Instant};

    let dirs: Vec<PathBuf> = dirs.into_iter().cloned().collect();
    let mut sample_registry = Vec::<FsSample>::new();
    let mut stack = dirs.clone();
    let mut time = Instant::now();

    while let Some(current_dir) = stack.pop() {
        if time.elapsed() >= Duration::from_millis(398) {
            sender.send_msg("s_tick", sample_registry.len().to_string());
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

    sender.send_msg("s_tick", sample_registry.len().to_string());
    tracing::info!("scan completed");

    let mut guard = app_state.write()?;

    match merge {
        ScanMerge::ReplaceAll => {
            guard.sample_registry = sample_registry
                .into_iter()
                .map(|s| (s.path.clone(), s))
                .collect();
        }
        ScanMerge::ReplaceUnder(roots) => {
            guard
                .sample_registry
                .retain(|path, _| !roots.iter().any(|root| path.starts_with(root)));
            guard
                .sample_registry
                .extend(sample_registry.into_iter().map(|s| (s.path.clone(), s)));
        }
    }

    Ok(())
}
