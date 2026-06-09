use std::sync::Arc;

use sample_model::wire::*;
use sample_model::*;
use serde::Serialize;

use crate::state::AppState;

// SampleEntry trait

pub trait SampleEntryFav: SampleEntrySerialize {
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

impl<T: SampleEntrySerialize> SampleEntryFav for T {}

// PluginSample (wraps WireEntry + carries plugin id)

#[derive(Debug, Clone, Serialize)]
pub struct PluginSample {
    pub entry: WireEntry,
    pub plugin_id: PluginId,
    /// Cached so hash_key() can return a &str without alloc.
    pub url: String,
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

    fn tags(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        self.entry.tags()
    }

    fn get_score_str<'a>(&self) -> std::borrow::Cow<'_, str> {
        self.entry.get_score_str()
    }
}

impl SampleEntrySerialize for PluginSample {
    fn source(&self) -> SampleSource {
        SampleSource::Plugin {
            id: self.plugin_id.clone(),
            url: self.url.clone(),
        }
    }

    fn to_serialize(&self) -> Result<SampleSerialize, SampleDataError> {
        Ok(SampleSerialize {
            name: self.entry.name().to_string(),
            source: SampleSource::Plugin {
                id: self.plugin_id.clone(),
                url: self.url.clone(),
            },
            meta: SampleMetadata {
                description: self.entry.description().map(Arc::from),
                tags: self.entry.tags().map(Arc::from).collect(),
                bpm: self.entry.bpm,
                sample_type: self.entry.sample_type,
            },
        })
    }
}
