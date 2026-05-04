#[cfg(feature = "std")]
use std::path::PathBuf;
#[cfg(feature = "std")]
use std::sync::Arc;

#[cfg(feature = "std")]
use serde::Serialize;
#[cfg(feature = "std")]
use ts_rs::TS;

#[cfg(not(feature = "std"))]
use alloc::string::String;
#[cfg(not(feature = "std"))]
use alloc::string::ToString;
#[cfg(not(feature = "std"))]
use alloc::sync::Arc;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use crate::{SampleType, WireEntry};

#[cfg_attr(feature = "std", derive(Debug, Clone, Serialize, TS))]
#[cfg_attr(feature = "std", ts(export, rename = "SampleEntry"))]
pub struct SampleEntryBase {
    pub name: String,

    #[cfg(feature = "std")]
    pub path: Option<PathBuf>,

    #[cfg(not(feature = "std"))]
    pub path: Option<String>,

    #[cfg_attr(feature = "std", serde(flatten))]
    pub meta: SampleMetadata,
}

#[cfg_attr(feature = "std", derive(Debug, Clone, Serialize, TS))]
pub struct SampleMetadata {
    pub tags: Vec<Arc<str>>,
    pub description: Option<Arc<str>>,
    pub bpm: Option<u16>,
    #[cfg_attr(feature = "std", serde(rename = "type"))]
    pub sample_type: SampleType,
}

impl From<SampleEntryBase> for WireEntry {
    fn from(entry: SampleEntryBase) -> Self {
        let mut str_content = String::with_capacity(entry.name.len());

        str_content.push_str(&entry.name);
        let name_end = str_content.len();

        #[cfg(not(feature = "std"))]
        let path_str = entry.path.as_ref();
        #[cfg(feature = "std")]
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

        Self {
            str_content,
            name_end,
            path_end,
            description_end,
            tags_end,
            bpm: entry.meta.bpm,
            sample_type: entry.meta.sample_type,
        }
    }
}

impl From<&WireEntry> for SampleEntryBase {
    fn from(raw: &WireEntry) -> Self {
        let s = &raw.str_content;

        let name = s[0..raw.name_end].to_string();

        let path = if raw.path_end > raw.name_end {
            Some(s[raw.name_end..raw.path_end].into())
        } else {
            None
        };

        let description = if raw.description_end > raw.path_end {
            Some(s[raw.path_end..raw.description_end].to_string().into())
        } else {
            None
        };

        let tags = if raw.tags_end > raw.description_end {
            s[raw.description_end..raw.tags_end]
                .split(',')
                .filter(|t| !t.is_empty())
                .map(|t| t.to_string().into())
                .collect()
        } else {
            Vec::new()
        };

        Self {
            name,
            path,
            meta: SampleMetadata {
                description,
                tags,
                bpm: raw.bpm,
                sample_type: raw.sample_type,
            },
        }
    }
}

impl From<WireEntry> for SampleEntryBase {
    fn from(value: WireEntry) -> Self {
        Self::from(&value)
    }
}
