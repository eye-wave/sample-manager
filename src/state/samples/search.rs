use std::path::Path;

use fuzzy_matcher::skim::SkimMatcherV2;
use rayon::{iter::Either, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    AStr,
    state::{AppState, samples::clean_up_string},
};

#[derive(Debug, Deserialize, Serialize)]
pub struct SearchRequest {
    #[serde(rename = "q")]
    query: AStr,
    #[serde(rename = "lim")]
    limit: usize,
    #[serde(rename = "off")]
    offset: usize,
    #[serde(rename = "t")]
    tags: AStr,
    #[serde(rename = "fav")]
    is_fav: bool,
}

#[allow(unused)]
#[derive(Deserialize)]
pub struct SampleResult {}

pub fn search(req: &SearchRequest, state: &AppState) -> String {
    let tags: Vec<&str> = req.tags.split(',').filter(|s| !s.is_empty()).collect();
    let query = clean_up_string(&req.query);

    let matcher = SkimMatcherV2::default().smart_case();

    let scored = if req.is_fav {
        Either::Left(state.favorite_samples.iter().filter_map(|f| {
            let key = Path::new(f);
            state.sample_registry.get(key)
        }))
    } else {
        Either::Right(state.sample_registry.values())
    };

    let mut result = scored
        .par_bridge()
        .map(|s| {
            if req.is_fav && query.is_empty() {
                return (s, i64::MAX);
            }

            let score = s.score(&query, &tags, &matcher);
            (s, score)
        })
        .filter(|(_, score)| *score > 0)
        .collect::<Vec<_>>();

    result.sort_by_key(|&(_, score)| std::cmp::Reverse(score));

    let start = req.offset;
    let end = (start + req.limit).min(result.len());

    let found = if start < result.len() {
        &result[start..end]
    } else {
        &[]
    };

    let files = found
        .iter()
        .map(|(f, _)| f.serialize(state.is_sample_fav(&f.path)))
        .intersperse(",\n".into())
        .collect::<String>();

    format!("{{\"count\":{},\"files\":[{files}]}}", result.len())
}
