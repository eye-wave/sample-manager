use std::path::Path;

use fuzzy_matcher::skim::SkimMatcherV2;
use plugin_wire::sample::SampleSerialize;
use rayon::{iter::Either, prelude::*};
use serde::Deserialize;
use ts_rs::TS;

use crate::plugins::PluginSendError;
use crate::state::{
    AppState,
    samples::{SampleEntry, clean_up_string},
};
use crate::{AStr, LogErrorExt};

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
    pub fn with_cleared_offsets(&self) -> Self {
        let mut partial = self.clone();

        partial.offset = 0;
        partial.limit = 0;

        partial
    }
}

pub fn search_local(req: &SearchRequest, state: &AppState) -> Result<String, PluginSendError> {
    let no_offset_req = req.with_cleared_offsets();
    let plugin_filtered = state.plugin_handle.search_local_registry(&no_offset_req)?;

    let items = if req.is_fav {
        Either::Left(state.favorite_samples.iter().filter_map(|f| {
            let key = Path::new(f);
            state.sample_registry.get(key)
        }))
    } else {
        Either::Right(state.sample_registry.values())
    };

    let (count1, found_local) = filter_samples(items.par_bridge(), &no_offset_req);

    let (count2, found) = filter_samples_dyn(
        plugin_filtered
            .iter()
            .filter(|e| !req.is_fav || e.is_fav(state))
            .map(|e| e as &dyn SampleEntry)
            .chain(found_local.iter().map(|e| *e as &dyn SampleEntry))
            .par_bridge(),
        &no_offset_req,
    );

    let count = count1 + count2;
    let found = slice_output(&found, req);

    let found = found
        .iter()
        .filter_map(|e| e.to_json(state).sure("Failed to serialize to json"))
        .intersperse(",\n".into())
        .collect::<String>();

    Ok(format!(r#"{{"count":{count},"files":[{found}]}}"#))
}

fn slice_output<'a, T>(slice: &'a [T], req: &'a SearchRequest) -> &'a [T] {
    if req.limit == 0 {
        return slice;
    }

    let start = req.offset.min(slice.len());
    let end = (start + req.limit).min(slice.len());

    &slice[start..end]
}

fn process_and_slice_entries<I, T, O>(
    entries: I,
    req: &SearchRequest,
    map_to_output: impl Fn(I::Item) -> (T, O) + Sync + Send,
) -> (usize, Vec<O>)
where
    I: ParallelIterator,
    I::Item: Send,
    T: AsAsDynSampleEntry + Send,
    O: Clone + Send,
{
    let query = clean_up_string(&req.query);
    let matcher = SkimMatcherV2::default().smart_case();
    let tag_refs: Vec<&str> = req.tags.iter().map(|s| s.as_ref()).collect();

    let mut result: Vec<(O, i64)> = entries
        .map(|item| {
            let (sample_ref, output_data) = map_to_output(item);
            let s = sample_ref.as_dyn_sample_entry();

            let score = if req.is_fav && query.is_empty() {
                i64::MAX
            } else {
                s.score(&query, &tag_refs, &matcher)
            };

            (output_data, score)
        })
        .filter(|(_, score)| *score > 0)
        .collect();

    result.sort_by_key(|&(_, score)| std::cmp::Reverse(score));

    let sliced = slice_output(&result, req);
    let sliced = sliced.iter().map(|(out, _)| out.clone()).collect();

    (result.len(), sliced)
}

pub fn filter_samples<'a, T: SampleEntry + Sized>(
    entries: impl ParallelIterator<Item = &'a T>,
    req: &SearchRequest,
) -> (usize, Vec<&'a T>) {
    process_and_slice_entries(entries, req, |s| (s, s))
}

pub fn filter_samples_dyn<'a>(
    entries: impl ParallelIterator<Item = &'a dyn SampleEntry>,
    req: &SearchRequest,
) -> (usize, Vec<SampleSerialize>) {
    process_and_slice_entries(entries, req, |s| (s, s.to_base()))
}

trait AsAsDynSampleEntry {
    fn as_dyn_sample_entry(&self) -> &dyn SampleEntry;
}

impl<T: SampleEntry + Sized> AsAsDynSampleEntry for &T {
    fn as_dyn_sample_entry(&self) -> &dyn SampleEntry {
        *self as &dyn SampleEntry
    }
}

impl AsAsDynSampleEntry for &dyn SampleEntry {
    fn as_dyn_sample_entry(&self) -> &dyn SampleEntry {
        *self
    }
}
