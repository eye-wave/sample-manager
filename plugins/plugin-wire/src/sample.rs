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
pub struct SampleSerialize {
    pub name: String,
    pub path: Option<String>,
    pub url: Option<String>,
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

impl From<SampleSerialize> for WireEntry {
    fn from(entry: SampleSerialize) -> Self {
        let mut str_content = String::with_capacity(entry.name.len());

        str_content.push_str(&entry.name);
        let name_end = str_content.len();

        if let Some(p) = &entry.path {
            str_content.push_str(p);
        }
        let path_end = str_content.len();

        if let Some(u) = &entry.url {
            str_content.push_str(u);
        }
        let url_end = str_content.len();

        if let Some(d) = entry.meta.description.as_ref() {
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
            url_end,
            description_end,
            tags_end,
            bpm: entry.meta.bpm,
            sample_type: entry.meta.sample_type,
        }
    }
}

impl From<&WireEntry> for SampleSerialize {
    fn from(raw: &WireEntry) -> Self {
        let s = &raw.str_content;

        let name = s[..raw.name_end].to_string();

        let path = if raw.path_end > raw.name_end {
            Some(s[raw.name_end..raw.path_end].to_string())
        } else {
            None
        };

        let url = if raw.url_end > raw.path_end {
            Some(s[raw.path_end..raw.url_end].to_string())
        } else {
            None
        };

        let description = if raw.description_end > raw.url_end {
            Some(s[raw.url_end..raw.description_end].to_string().into())
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
            url,
            meta: SampleMetadata {
                description,
                tags,
                bpm: raw.bpm,
                sample_type: raw.sample_type,
            },
        }
    }
}

impl From<WireEntry> for SampleSerialize {
    fn from(value: WireEntry) -> Self {
        Self::from(&value)
    }
}
