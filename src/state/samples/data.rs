use std::{fmt::Debug, sync::Arc};

use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use plugin_wire::{
    WireEntry,
    sample::{SampleSerialize, SampleSource},
};
use serde::Serialize;
use ts_rs::TS;

use crate::state::AppState;

#[derive(Serialize, TS)]
#[ts(export, rename = "SampleEntry")]
struct WithFav {
    is_fav: bool,
    #[serde(flatten)]
    inner: SampleSerialize,
}

pub trait SampleEntry: Sync + Debug {
    fn score(&self, query: &str, tags: &[&str], matcher: &SkimMatcherV2) -> i64;
    fn source(&self) -> SampleSource;

    fn path(&self) -> Option<&str>;
    fn url(&self) -> Option<&str>;

    fn to_hash(&self) -> Option<&str> {
        match self.source() {
            SampleSource::Native => self.path(),
            SampleSource::Plugin => self.url(),
        }
    }

    fn to_base(&self) -> SampleSerialize;

    fn is_fav(&self, state: &AppState) -> bool {
        if let Some(hash) = self.to_hash() {
            state.favorite_samples.contains(hash)
        } else {
            false
        }
    }

    fn to_json(&self, state: &AppState) -> Result<String, serde_json::Error> {
        let is_fav = self.is_fav(state);

        serde_json::to_string(&WithFav {
            is_fav,
            inner: self.to_base(),
        })
    }
}

impl SampleEntry for WireEntry {
    fn source(&self) -> SampleSource {
        SampleSource::Plugin
    }

    fn path(&self) -> Option<&str> {
        self.path()
    }

    fn url(&self) -> Option<&str> {
        self.url()
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

    fn to_base(&self) -> SampleSerialize {
        self.into()
    }
}

impl SampleEntry for SampleSerialize {
    fn score(&self, query: &str, tags: &[&str], matcher: &SkimMatcherV2) -> i64 {
        if !tags.is_empty() {
            let has_all = tags.iter().all(|t| self.meta.tags.contains(&Arc::from(*t)));
            if !has_all {
                return i64::MIN;
            }
        }

        let search_str = self.name.clone()
            + self.url.as_ref().unwrap_or(&"".to_string())
            + self.path.as_ref().unwrap_or(&"".to_string());

        matcher.fuzzy_match(&search_str, query).unwrap_or(i64::MIN)
    }

    fn to_base(&self) -> SampleSerialize {
        self.clone()
    }

    fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    fn source(&self) -> SampleSource {
        self.source.clone()
    }
}
