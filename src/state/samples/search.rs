use std::path::Path;

use fuzzy_matcher::skim::SkimMatcherV2;
use rayon::{iter::Either, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    AStr,
    state::{
        AppState,
        samples::{SampleEntry, clean_up_string},
    },
};

#[derive(Debug, Deserialize, Serialize)]
pub struct SearchRequest {
    pub query: AStr,
    pub limit: usize,
    pub offset: usize,
    pub tags: Vec<AStr>,
    pub is_fav: bool,
}

pub fn search_local(req: &SearchRequest, state: &AppState) -> String {
    let scored = if req.is_fav {
        Either::Left(state.favorite_samples.iter().filter_map(|f| {
            let key = Path::new(f);
            state.sample_registry.get(key)
        }))
    } else {
        Either::Right(state.sample_registry.values())
    };

    let found = filter_samples(scored.par_bridge(), req);

    let files = found
        .iter()
        .map(|f| f.serialize(state.is_sample_fav(&f.path)))
        .intersperse(",\n".into())
        .collect::<String>();

    format!("{{\"count\":{},\"files\":[{files}]}}", found.len())
}

pub fn filter_samples<'a, T: SampleEntry + Sync>(
    entries: impl ParallelIterator<Item = &'a T>,
    req: &SearchRequest,
) -> Vec<&'a T> {
    let query = clean_up_string(&req.query);
    let matcher = SkimMatcherV2::default().smart_case();

    let mut result: Vec<(&T, i64)> = entries
        .map(|s| {
            let score = if req.is_fav && query.is_empty() {
                i64::MAX
            } else {
                s.score(&query, &req.tags, &matcher)
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
