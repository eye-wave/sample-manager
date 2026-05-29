use std::cmp::Reverse;
use std::path::Path;

use fuzzy_matcher::skim::SkimMatcherV2;
use rayon::{iter::Either, prelude::*};
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

impl SearchRequest {
    fn without_pagination(&self) -> Self {
        Self {
            offset: 0,
            limit: 0,
            ..self.clone()
        }
    }
}

pub fn search_local(req: &SearchRequest, state: &AppState) -> Result<String, PluginSendError> {
    let unpaged = req.without_pagination();

    let native_iter = if req.is_fav {
        Either::Left(
            state
                .favorite_samples
                .iter()
                .filter_map(|f| state.sample_registry.get(Path::new(f))),
        )
    } else {
        Either::Right(state.sample_registry.values())
    };

    let (native_count, native_hits) = score_and_sort(native_iter.par_bridge(), &unpaged);

    let plugin_registry = state.plugin_handle.search_local_registry(&unpaged)?;
    let plugin_iter = plugin_registry
        .par_iter()
        .flat_map(|a| a.par_iter())
        .filter(|e| !req.is_fav || e.is_fav(state).unwrap_or(false))
        .map(|e| e as &dyn SampleEntry);

    let (plugin_count, plugin_hits) = score_and_sort(plugin_iter, &unpaged);

    let total = native_count + plugin_count;

    let mut merged: Vec<SampleSerialize> = native_hits
        .into_iter()
        .filter_map(|e| e.to_serialize().ok())
        .chain(
            plugin_hits
                .into_iter()
                .filter_map(|e| e.to_serialize().ok()),
        )
        .collect();

    let query = clean_up_string(&req.query);
    let matcher = SkimMatcherV2::default().smart_case();
    let tag_refs: Vec<&str> = req.tags.iter().map(|s| s.as_ref()).collect();
    merged.sort_by_key(|s| {
        Reverse(if req.is_fav && query.is_empty() {
            i64::MAX
        } else {
            s.score(&query, &tag_refs, &matcher)
        })
    });

    let page = paginate(&merged, req);

    let body = page
        .iter()
        .filter_map(|e| e.to_json(state).sure("Failed to serialize"))
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

fn paginate<'a, T>(items: &'a [T], req: &SearchRequest) -> &'a [T] {
    if req.limit == 0 {
        return items;
    }
    let start = req.offset.min(items.len());
    let end = (start + req.limit).min(items.len());
    &items[start..end]
}
