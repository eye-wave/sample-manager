use std::cmp::Reverse;
use std::path::Path;

use fuzzy_matcher::skim::SkimMatcherV2;
use rayon::prelude::*;
use serde::Deserialize;
use ts_rs::TS;

use crate::AStr;
use crate::LogErrorExt;
use crate::plugins::PluginSendError;
use crate::state::{
    AppState,
    samples::{SampleEntry, SampleSerialize, clean_up_string},
};

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

pub fn search_local(req: &SearchRequest, state: &AppState) -> Result<String, PluginSendError> {
    let query = clean_up_string(&req.query);
    let matcher = SkimMatcherV2::default().smart_case();
    let tag_refs: Vec<&str> = req.tags.iter().map(|s| s.as_ref()).collect();

    let score_fn = |item: &dyn SampleEntry| -> i64 {
        if req.is_fav && query.is_empty() {
            i64::MAX
        } else {
            item.score(&query, &tag_refs, &matcher)
        }
    };

    let native_vec: Vec<_> = if req.is_fav {
        state
            .favorite_samples
            .iter()
            .filter_map(|f| state.sample_registry.get(Path::new(f)))
            .collect()
    } else {
        state.sample_registry.values().collect()
    };

    let mut native_scored: Vec<(SampleSerialize, i64)> = native_vec
        .par_iter()
        .filter_map(|item| {
            let score = score_fn(*item as &dyn SampleEntry);
            if score > i64::MIN {
                item.to_serialize().ok().map(|s| (s, score))
            } else {
                None
            }
        })
        .collect();

    let plugin_registry = state.plugin_handle.search_local_registry(req)?;
    let mut plugin_scored: Vec<(SampleSerialize, i64)> = plugin_registry
        .par_iter()
        .flat_map(|a| a.par_iter())
        .filter(|e| !req.is_fav || e.is_fav(state).unwrap_or(false))
        .filter_map(|item| {
            let score = score_fn(item as &dyn SampleEntry);
            if score > i64::MIN {
                item.to_serialize().ok().map(|s| (s, score))
            } else {
                None
            }
        })
        .collect();

    let total = native_scored.len() + plugin_scored.len();

    native_scored.append(&mut plugin_scored);
    native_scored.sort_unstable_by_key(|&(_, s)| Reverse(s));

    let start = req.offset.min(total);
    let end = if req.limit == 0 {
        total
    } else {
        (start + req.limit).min(total)
    };

    let body = native_scored[start..end]
        .iter()
        .filter_map(|(s, _)| s.to_json(state).sure("Failed to serialize"))
        .intersperse(",\n".into())
        .collect::<String>();

    Ok(format!(r#"{{"count":{total},"files":[{body}]}}"#))
}

pub fn score_and_sort<'a, T>(
    iter: impl ParallelIterator<Item = &'a T>,
    req: &SearchRequest,
) -> (usize, Vec<&'a T>)
where
    T: SampleEntry + ?Sized + 'a,
{
    let query = clean_up_string(&req.query);
    let matcher = SkimMatcherV2::default().smart_case();
    let tag_refs: Vec<&str> = req.tags.iter().map(|s| s.as_ref()).collect();

    let mut scored: Vec<(&T, i64)> = iter
        .map(|item| {
            let score = if req.is_fav && query.is_empty() {
                i64::MAX
            } else {
                item.score(&query, &tag_refs, &matcher)
            };
            (item, score)
        })
        .filter(|(_, score)| *score > i64::MIN)
        .collect();

    scored.sort_by_key(|&(_, s)| Reverse(s));

    let count = scored.len();
    let items = scored.into_iter().map(|(item, _)| item).collect();
    (count, items)
}

pub fn filter_samples<'a, T: SampleEntry + Sized>(
    entries: impl ParallelIterator<Item = &'a T>,
    req: &SearchRequest,
) -> (usize, Vec<&'a T>) {
    score_and_sort(entries, req)
}
