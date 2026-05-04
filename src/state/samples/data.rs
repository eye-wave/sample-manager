use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use plugin_wire::WireEntry;

pub trait SampleEntry: Sync {
    // fn name(&self) -> &str;
    // fn path(&self) -> &Path;
    // fn meta(&self) -> SampleMetadataRef<'_, impl Iterator<Item = &str>>;
    fn score<T: AsRef<str>>(&self, query: &str, tags: &[T], matcher: &SkimMatcherV2) -> i64;
}

impl SampleEntry for WireEntry {
    // fn name(&self) -> &str {
    //     &self.str_content[0..self.indices.name_end]
    // }

    // fn path(&self) -> &Path {
    //     Path::new(&self.str_content[self.indices.name_end..self.indices.path_end])
    // }

    // fn meta(&self) -> SampleMetadataRef<'_, impl Iterator<Item = &str>> {
    //     let description = &self.str_content[self.indices.path_end..self.indices.description_end];

    //     SampleMetadataRef {
    //         tags: self.str_content[self.indices.description_end..self.indices.tags_end].split(","),
    //         description,
    //         bpm: self.bpm,
    //         sample_type: self.sample_type.clone(),
    //     }
    // }

    fn score<T: AsRef<str>>(&self, query: &str, tags: &[T], matcher: &SkimMatcherV2) -> i64 {
        if !tags.is_empty() {
            let has_all = tags
                .iter()
                .all(|t1| self.tags().into_iter().any(|t2| t1.as_ref() == t2));
            if !has_all {
                return i64::MIN;
            }
        }

        matcher
            .fuzzy_match(&self.str_content, query)
            .unwrap_or(i64::MIN)
    }
}

// pub struct SampleMetadataRef<'a, I>
// where
//     I: Iterator<Item = &'a str>,
// {
//     pub tags: I,
//     pub description: &'a str,
//     pub bpm: Option<u16>,
//     pub sample_type: SampleType,
// }
