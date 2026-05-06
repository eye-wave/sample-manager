use std::{borrow::Cow, sync::Arc};

use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use plugin_wire::{WireEntry, sample::SampleEntryBase};
use serde::Serialize;
use ts_rs::TS;

use crate::state::AppState;

#[derive(Serialize, TS)]
#[ts(export, rename = "SampleEntry")]
struct WithFav {
    is_fav: bool,
    #[serde(flatten)]
    inner: SampleEntryBase,
}

pub trait SampleEntry: Sync {
    fn score(&self, query: &str, tags: &[&str], matcher: &SkimMatcherV2) -> i64;

    fn to_hash<'a>(&'a self) -> Cow<'a, str>;
    fn to_base(&self) -> SampleEntryBase;

    fn is_fav(&self, state: &AppState) -> bool {
        state.favorite_samples.contains(&self.to_hash().to_string())
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
    fn to_hash(&self) -> Cow<'_, str> {
        Cow::Owned(self.name().to_string() + self.url().unwrap_or(""))
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

    fn to_base(&self) -> SampleEntryBase {
        self.into()
    }
}

impl SampleEntry for SampleEntryBase {
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

    fn to_hash<'a>(&'a self) -> Cow<'a, str> {
        Cow::Owned(
            self.name.clone()
                + self.url.as_ref().unwrap_or(&"".to_string())
                + self.path.as_ref().unwrap_or(&"".to_string()),
        )
    }

    fn to_base(&self) -> SampleEntryBase {
        self.clone()
    }
}
