use std::borrow::Cow;

use heapless::vec::Vec as HVec;
use serde::{Deserialize, Serialize};

use crate::{SampleDataError, SampleEntry};

use super::SampleType;

pub const WIRE_MAX_TAG_COUNT: usize = 24;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireEntry {
    str_content: String,
    name_end: u16,
    path_end: u16,
    url_end: u16,
    description_end: u16,
    pub tags: HVec<String, WIRE_MAX_TAG_COUNT>,
    pub bpm: Option<u16>,
    pub sample_type: SampleType,
}

impl WireEntry {
    pub fn name(&self) -> &str {
        &self.str_content[0..self.name_end as usize]
    }

    pub fn path(&self) -> Option<&str> {
        let start = self.name_end as usize;
        let end = self.path_end as usize;
        if start == end {
            None
        } else {
            Some(&self.str_content[start..end])
        }
    }

    pub fn url(&self) -> Option<&str> {
        let start = self.path_end as usize;
        let end = self.url_end as usize;
        if start == end {
            None
        } else {
            Some(&self.str_content[start..end])
        }
    }

    pub fn description(&self) -> Option<&str> {
        let start = self.url_end as usize;
        let end = self.description_end as usize;
        if start == end {
            None
        } else {
            Some(&self.str_content[start..end])
        }
    }
}

pub fn write_frame(entries: &[WireEntry]) -> Result<Vec<u8>, postcard::Error> {
    let bytes = postcard::to_allocvec(entries)?;
    let mut size = bytes.len().to_be_bytes().to_vec();

    size.extend(bytes.iter());
    Ok(size)
}

#[derive(Debug, thiserror::Error)]
pub enum FrameParseError {
    #[error("Input is too short")]
    TooShort,

    #[error("{0}")]
    Postcard(#[from] postcard::Error),
}

pub fn parse_frame(bytes: &[u8]) -> Result<(Vec<WireEntry>, usize), FrameParseError> {
    if bytes.len() < 4 {
        return Err(FrameParseError::TooShort);
    }

    let size = u32::from_be_bytes(bytes[0..4].try_into().unwrap()) as usize;

    if bytes.len() < 4 + size {
        return Err(FrameParseError::TooShort);
    }

    let entries = postcard::from_bytes(&bytes[4..4 + size])?;
    Ok((entries, size))
}

impl std::hash::Hash for WireEntry {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name().hash(state);
        self.url().hash(state);
    }
}

impl PartialEq for WireEntry {
    fn eq(&self, other: &Self) -> bool {
        self.name() == other.name() && self.url() == other.url()
    }
}

impl Eq for WireEntry {}

impl WireEntry {
    pub fn new(
        name: &str,
        path: &str,
        url: &str,
        description: &str,
        tags_src: &[&str],
        bpm: Option<u16>,
        sample_type: SampleType,
    ) -> Self {
        let mut str_content =
            String::with_capacity(name.len() + path.len() + url.len() + description.len());

        str_content.push_str(name);
        let name_end = str_content.len() as u16;

        str_content.push_str(path);
        let path_end = str_content.len() as u16;

        str_content.push_str(url);
        let url_end = str_content.len() as u16;

        str_content.push_str(description);
        let description_end = str_content.len() as u16;

        let mut tags = HVec::new();

        for &tag in tags_src.iter().take(WIRE_MAX_TAG_COUNT) {
            let _ = tags.push(tag.to_string());
        }

        Self {
            str_content,
            name_end,
            path_end,
            url_end,
            description_end,
            tags,
            bpm,
            sample_type,
        }
    }
}

impl SampleEntry for WireEntry {
    fn hash_key(&self) -> Result<&str, SampleDataError> {
        self.url().ok_or(SampleDataError::MissingUrl {
            info: self.str_content.to_string(),
        })
    }

    fn tags(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        Box::new(self.tags.iter().map(|s| s.as_ref()))
    }

    fn get_score_str<'a>(&self) -> std::borrow::Cow<'_, str> {
        Cow::Borrowed(self.str_content.as_str())
    }
}
