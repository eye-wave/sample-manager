use std::sync::Arc;

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use plugin_wire::{WireEntry, sample::SampleMetadata};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::plugins::PluginId;
use crate::state::AppState;

#[derive(Debug, thiserror::Error)]
pub enum SampleDataError {
    #[error("Sample has missing url. details: [{info}]")]
    MissingUrl { info: String },

    #[error("Failed to convert Path to &str")]
    PathConversionError,

    #[error("Cannot inherit plugin id from this sample. details: [{info}]")]
    NoPluginId { info: String },

    #[error("{0}")]
    Serde(#[from] serde_json::Error),
}

// SampleSource

#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum SampleSource {
    Native { path: String },
    Plugin { id: PluginId, url: String },
}

impl SampleSource {
    pub fn hash_key(&self) -> &str {
        match self {
            Self::Native { path } => path.as_str(),
            Self::Plugin { url, .. } => url.as_str(),
        }
    }
}

// SampleSerialize  (wire type sent to the frontend)

#[derive(Debug, Clone, Serialize, TS)]
pub struct SampleSerialize {
    #[serde(flatten)]
    pub source: SampleSource,
    pub name: String,
    #[serde(flatten)]
    pub meta: SampleMetadata,
}

#[derive(Serialize, TS)]
#[ts(export, rename = "SampleEntry")]
#[serde(rename_all = "camelCase")]
struct SampleWithFav {
    is_fav: bool,
    #[serde(flatten)]
    inner: SampleSerialize,
}

// SampleEntry trait

pub trait SampleEntry: std::fmt::Debug + Sync {
    fn hash_key(&self) -> Result<&str, SampleDataError>;
    fn score(&self, query: &str, tags: &[&str], matcher: &SkimMatcherV2) -> i64;
    fn to_serialize(&self) -> Result<SampleSerialize, SampleDataError>;

    fn is_fav(&self, state: &AppState) -> Result<bool, SampleDataError> {
        Ok(state.favorite_samples.contains(self.hash_key()?))
    }

    fn to_json(&self, state: &AppState) -> Result<String, SampleDataError> {
        Ok(serde_json::to_string(&SampleWithFav {
            is_fav: self.is_fav(state)?,
            inner: self.to_serialize()?,
        })?)
    }
}

// PluginSample  (wraps WireEntry + carries plugin id)

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginSample {
    pub entry: WireEntry,
    pub plugin_id: PluginId,
    /// Cached so hash_key() can return a &str without alloc.
    url: String,
}

impl PluginSample {
    pub fn new(entry: WireEntry, plugin_id: PluginId) -> Self {
        let url = entry.url().unwrap_or_default().to_owned();
        Self {
            entry,
            plugin_id,
            url,
        }
    }
}

impl SampleEntry for PluginSample {
    fn hash_key(&self) -> Result<&str, SampleDataError> {
        Ok(&self.url)
    }

    fn score(&self, query: &str, tags: &[&str], matcher: &SkimMatcherV2) -> i64 {
        if !tags.is_empty() {
            let has_all = tags
                .iter()
                .all(|t1| self.entry.tags().into_iter().any(|t2| *t1 == t2));
            if !has_all {
                return i64::MIN;
            }
        }
        matcher
            .fuzzy_match(&self.entry.str_content, query)
            .unwrap_or(i64::MIN)
    }

    fn to_serialize(&self) -> Result<SampleSerialize, SampleDataError> {
        let e = &self.entry;
        let s = &e.str_content;

        let name = s[..e.name_end].to_string();

        let url = if e.url_end > e.path_end {
            s[e.path_end..e.url_end].to_string()
        } else {
            self.url.clone()
        };

        let description =
            (e.description_end > e.url_end).then(|| Arc::from(&s[e.url_end..e.description_end]));

        let tags = if e.tags_end > e.description_end {
            s[e.description_end..e.tags_end]
                .split(',')
                .filter(|t| !t.is_empty())
                .map(Arc::from)
                .collect()
        } else {
            Vec::new()
        };

        Ok(SampleSerialize {
            name,
            source: SampleSource::Plugin {
                id: self.plugin_id.clone(),
                url,
            },
            meta: SampleMetadata {
                description,
                tags,
                bpm: e.bpm,
                sample_type: e.sample_type,
            },
        })
    }
}

// SampleSerialize as a SampleEntry (used when re-scoring already-serialized
// results from the plugin local registry)

impl SampleEntry for SampleSerialize {
    fn hash_key(&self) -> Result<&str, SampleDataError> {
        Ok(self.source.hash_key())
    }

    fn score(&self, query: &str, tags: &[&str], matcher: &SkimMatcherV2) -> i64 {
        if !tags.is_empty() {
            let has_all = tags.iter().all(|t| self.meta.tags.contains(&Arc::from(*t)));
            if !has_all {
                return i64::MIN;
            }
        }

        let search_str = match &self.source {
            SampleSource::Native { path } => self.name.clone() + path,
            SampleSource::Plugin { url, .. } => self.name.clone() + url,
        };

        matcher.fuzzy_match(&search_str, query).unwrap_or(i64::MIN)
    }

    fn to_serialize(&self) -> Result<SampleSerialize, SampleDataError> {
        Ok(self.clone())
    }
}

// WireEntry

impl SampleEntry for WireEntry {
    fn hash_key(&self) -> Result<&str, SampleDataError> {
        self.url().ok_or(SampleDataError::MissingUrl {
            info: self.str_content.clone(),
        })
    }

    fn score(&self, query: &str, tags: &[&str], matcher: &SkimMatcherV2) -> i64 {
        if !tags.is_empty() {
            let has_all = tags
                .iter()
                .all(|t1| self.tags().into_iter().any(|t2| *t1 == t2));
            if !has_all {
                return i64::MIN;
            }
        }
        matcher
            .fuzzy_match(&self.str_content, query)
            .unwrap_or(i64::MIN)
    }

    fn to_serialize(&self) -> Result<SampleSerialize, SampleDataError> {
        Err(SampleDataError::NoPluginId {
            info: self.str_content.clone(),
        })
    }
}
