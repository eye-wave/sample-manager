use std::path::Path;

use fuzzy_matcher::skim::SkimMatcherV2;
use plugin_wire::sample::SampleEntryBase;
use rayon::{iter::Either, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    AStr, AnyResult,
    state::{
        AppState,
        samples::{SampleEntry, clean_up_string},
    },
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchRequest {
    pub query: AStr,
    pub limit: usize,
    pub offset: usize,
    pub tags: Vec<AStr>,
    pub is_fav: bool,
}

pub fn search_local(req: &SearchRequest, state: &AppState) -> AnyResult<String> {
    let plugin_filtered = state.plugin_handle.search_local_registry(req)?;

    let scored = if req.is_fav {
        Either::Left(state.favorite_samples.iter().filter_map(|f| {
            let key = Path::new(f);
            state.sample_registry.get(key)
        }))
    } else {
        Either::Right(state.sample_registry.values())
    };

    let found_local = filter_samples(scored.par_bridge(), req);
    let found = filter_samples_dyn(
        plugin_filtered
            .iter()
            .map(|e| e as &dyn SampleEntry)
            .chain(found_local.iter().map(|e| *e as &dyn SampleEntry))
            .par_bridge(),
        req,
    )
    .iter()
    .filter_map(|e| e.to_json(state).ok())
    .intersperse(",\n".into())
    .collect::<String>();

    Ok(format!(r#"{{"count":{},"files":[{found}]}}"#, found.len()))
}

pub fn filter_samples<'a, T: SampleEntry + Sized>(
    entries: impl ParallelIterator<Item = &'a T>,
    req: &SearchRequest,
) -> Vec<&'a T> {
    let query = clean_up_string(&req.query);
    let matcher = SkimMatcherV2::default().smart_case();

    let mut result: Vec<(&'a T, i64)> = entries
        .map(|s| {
            let score = if req.is_fav && query.is_empty() {
                i64::MAX
            } else {
                s.score(
                    &query,
                    &req.tags.iter().map(|s| s.as_ref()).collect::<Vec<_>>(),
                    &matcher,
                )
            };

            (s, score)
        })
        .filter(|(_, score)| *score > 0)
        .collect();

    result.sort_by_key(|&(_, score)| std::cmp::Reverse(score));

    let start = req.offset;
    let end = (start + req.limit).min(result.len());

    result[start..end].iter().map(|(s, _)| *s).collect()
}

pub fn filter_samples_dyn<'a>(
    entries: impl ParallelIterator<Item = &'a dyn SampleEntry>,
    req: &SearchRequest,
) -> Vec<SampleEntryBase> {
    let query = clean_up_string(&req.query);
    let matcher = SkimMatcherV2::default().smart_case();

    let mut result: Vec<(SampleEntryBase, i64)> = entries
        .map(|s| {
            let score = if req.is_fav && query.is_empty() {
                i64::MAX
            } else {
                s.score(
                    &query,
                    &req.tags.iter().map(|s| s.as_ref()).collect::<Vec<_>>(),
                    &matcher,
                )
            };

            (s.to_base(), score)
        })
        .filter(|(_, score)| *score > 0)
        .collect();

    result.sort_by_key(|&(_, score)| std::cmp::Reverse(score));

    let start = req.offset;
    let end = (start + req.limit).min(result.len());

    result[start..end].iter().map(|(s, _)| s.clone()).collect()
}
