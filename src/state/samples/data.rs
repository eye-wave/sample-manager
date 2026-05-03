use std::path::PathBuf;
// use std::path::Path;

use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use serde::Serialize;
use ts_rs::TS;

use crate::AStr;

pub trait SampleEntry: Sync {
    // fn name(&self) -> &str;
    // fn path(&self) -> &Path;
    // fn meta(&self) -> SampleMetadataRef<'_, impl Iterator<Item = &str>>;
    fn score<T: AsRef<str>>(&self, query: &str, tags: &[T], matcher: &SkimMatcherV2) -> i64;
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, rename = "SampleEntry")]
pub struct SampleEntrySerialize {
    name: String,
    path: Option<PathBuf>,
    #[serde(flatten)]
    meta: SampleMetadata,
}

pub struct RawSampleEntry {
    pub str_content: String,
    pub indices: RawSampleIndices,
    pub bpm: Option<u16>,
    pub sample_type: SampleType,
}

pub struct RawSampleIndices {
    pub name_end: usize,
    pub path_end: usize,
    pub description_end: usize,
    pub tags_end: usize,
}

impl RawSampleEntry {
    fn tags(&self) -> impl Iterator<Item = &str> {
        self.str_content[self.indices.description_end..self.indices.tags_end].split(',')
    }
}

impl SampleEntry for RawSampleEntry {
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

#[derive(Debug, Clone, Serialize, TS)]
pub struct SampleMetadata {
    pub tags: Vec<AStr>,
    pub description: Option<AStr>,
    pub bpm: Option<u16>,
    #[serde(rename = "type")]
    pub sample_type: SampleType,
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

#[derive(Debug, Clone, Serialize, TS)]
pub enum SampleType {
    OneShot,
    Loop,
}

impl From<SampleEntrySerialize> for RawSampleEntry {
    fn from(entry: SampleEntrySerialize) -> Self {
        let mut str_content = String::with_capacity(entry.name.len());

        str_content.push_str(&entry.name);
        let name_end = str_content.len();

        let path_str = entry.path.as_ref().map(|p| p.to_string_lossy());

        if let Some(p) = &path_str {
            str_content.push_str(p);
        }
        let path_end = str_content.len();

        let desc_str = entry.meta.description.as_ref();

        if let Some(d) = desc_str {
            str_content.push_str(d.as_ref());
        }
        let description_end = str_content.len();

        let tags_str = entry
            .meta
            .tags
            .iter()
            .map(|t| t.as_ref())
            .collect::<Vec<_>>()
            .join(",");

        str_content.push_str(&tags_str);
        let tags_end = str_content.len();

        RawSampleEntry {
            str_content,
            indices: RawSampleIndices {
                name_end,
                path_end,
                description_end,
                tags_end,
            },
            bpm: entry.meta.bpm,
            sample_type: entry.meta.sample_type,
        }
    }
}

impl From<&RawSampleEntry> for SampleEntrySerialize {
    fn from(raw: &RawSampleEntry) -> Self {
        let s = &raw.str_content;
        let idx = &raw.indices;

        let name = s[0..idx.name_end].to_string();

        let path = if idx.path_end > idx.name_end {
            Some(s[idx.name_end..idx.path_end].into())
        } else {
            None
        };

        let description = if idx.description_end > idx.path_end {
            Some(s[idx.path_end..idx.description_end].to_string().into())
        } else {
            None
        };

        let tags = if idx.tags_end > idx.description_end {
            s[idx.description_end..idx.tags_end]
                .split(',')
                .filter(|t| !t.is_empty())
                .map(|t| t.to_string().into())
                .collect()
        } else {
            Vec::new()
        };

        SampleEntrySerialize {
            name,
            path,
            meta: SampleMetadata {
                description,
                tags,
                bpm: raw.bpm,
                sample_type: raw.sample_type.clone(),
            },
        }
    }
}

impl From<RawSampleEntry> for SampleEntrySerialize {
    fn from(value: RawSampleEntry) -> Self {
        Self::from(&value)
    }
}
