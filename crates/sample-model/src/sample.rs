use std::{borrow::Cow, sync::Arc};

use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::wire::SampleType;

type AStr = Arc<str>;

#[derive(Debug, thiserror::Error)]
pub enum SampleDataError {
    #[error("Sample has missing url. details: [{info}]")]
    MissingUrl { info: String },

    #[error("Failed to convert Path to &str")]
    PathConversionError,

    #[error("{0}")]
    Serde(#[from] serde_json::Error),
}

#[derive(Clone, Debug, Serialize, TS, PartialEq, Eq, Hash)]
pub struct PluginId(AStr);

#[derive(Debug, thiserror::Error)]
#[error("invalid plugin id: {0}")]
pub struct PluginIdError(String);

impl PluginId {
    pub fn new(str: impl AsRef<str>) -> Result<Self, PluginIdError> {
        const FORBIDDEN: &[char] = &['<', '>', ':'];
        let s = str.as_ref();

        if s == "__APP_SETTINGS__"
            || s.chars()
                .any(|c| c.is_whitespace() || FORBIDDEN.contains(&c))
        {
            return Err(PluginIdError(str.as_ref().to_owned()));
        }

        Ok(Self(Arc::from(s)))
    }
}

impl std::fmt::Display for PluginId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::ops::Deref for PluginId {
    type Target = AStr;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> Deserialize<'de> for PluginId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        PluginId::new(s.as_str()).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum SampleSource {
    Native { path: String },
    Plugin { id: PluginId, url: String },
}

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
pub struct SampleWithFav {
    pub is_fav: bool,
    #[serde(flatten)]
    pub inner: SampleSerialize,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct SampleMetadata {
    pub tags: Vec<AStr>,
    pub description: Option<AStr>,
    pub bpm: Option<u16>,
    pub sample_type: SampleType,
}

pub trait SampleEntry: std::fmt::Debug + Sync {
    fn hash_key(&self) -> Result<&str, SampleDataError>;
    fn tags(&self) -> Box<dyn Iterator<Item = &str> + '_>;

    fn get_score_str(&self) -> Cow<'_, str>;
    fn score(&self, query: &str, tags: &[&str], matcher: &SkimMatcherV2) -> i64 {
        if !tags.is_empty() {
            let has_all = tags.iter().all(|t1| self.tags().any(|t2| *t1 == t2));
            if !has_all {
                return i64::MIN;
            }
        }

        matcher
            .fuzzy_match(&self.get_score_str(), query)
            .unwrap_or(i64::MIN)
    }
}

pub trait SampleEntrySerialize: SampleEntry {
    fn source(&self) -> SampleSource;
    fn to_serialize(&self) -> Result<SampleSerialize, SampleDataError>;
}

impl SampleSource {
    pub fn from_plug(id: PluginId, url: String) -> Self {
        Self::Plugin {
            id: id.clone(),
            url,
        }
    }

    pub fn from_native(path: String) -> Self {
        Self::Native { path }
    }

    pub fn hash_key(&self) -> &str {
        match self {
            Self::Native { path } => path.as_str(),
            Self::Plugin { url, .. } => url.as_str(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SearchRequest {
    pub query: AStr,
    pub limit: usize,
    pub offset: usize,
    pub tags: Vec<AStr>,
    pub is_fav: bool,
}

impl SampleEntry for SampleSerialize {
    fn hash_key(&self) -> Result<&str, SampleDataError> {
        Ok(self.source.hash_key())
    }

    fn tags(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        Box::new(self.meta.tags.iter().map(|s| s.as_ref()))
    }

    fn get_score_str(&self) -> Cow<'_, str> {
        match &self.source {
            SampleSource::Native { path } => self.name.clone() + path,
            SampleSource::Plugin { url, .. } => self.name.clone() + url,
        }
        .into()
    }
}

impl SampleEntrySerialize for SampleSerialize {
    fn source(&self) -> SampleSource {
        self.source.clone()
    }

    fn to_serialize(&self) -> Result<SampleSerialize, SampleDataError> {
        Ok(self.clone())
    }
}
