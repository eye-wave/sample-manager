use crate::types::SampleType;
#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// -- byte offsets within a record's fixed header -------------------------------
//
// Result frame layout (little-endian, plugin → host):
//   [0..4]   u32  record_count
//   [4..]    Record * record_count
//
// Record layout:
//   [+0 .. +4]          u32  str_len
//   [+4 .. +8]          u32  name_end
//   [+8 .. +12]         u32  path_end
//   [+12.. +16]         u32  url_end
//   [+16.. +20]         u32  description_end
//   [+20.. +24]         u32  tags_end
//   [+24.. +26]         u16  bpm  (0 when has_bpm = 0)
//   [+26]               u8   has_bpm  (0 or 1)
//   [+27]               u8   sample_type
//   [+28.. +28+str_len] u8[str_len]  str_content utf-8
//
// str_content packs: name | path | url | description | comma-separated tags
// The five _end offsets are byte positions within str_content.

const OFF_STR_LEN: usize = 0;
const OFF_NAME_END: usize = 4;
const OFF_PATH_END: usize = 8;
const OFF_URL_END: usize = 12;
const OFF_DESC_END: usize = 16;
const OFF_TAGS_END: usize = 20;
const OFF_BPM: usize = 24;
const OFF_HAS_BPM: usize = 26;
const OFF_STYPE: usize = 27;
const REC_FIXED: usize = 28;
const FRAME_HEADER: usize = 4; // u32 record_count

// -- WireEntry — the in-memory representation of one record --------------------

/// One sample entry in the wire format, ready to be written or just parsed.
///
/// `str_content` is the packed string: `name || path || url || description || tags`
/// where `||` means concatenation. The five `*_end` fields are byte offsets
/// into `str_content` marking where each section ends.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct WireEntry {
    /// name[0..name_end] | path[name_end..path_end] | url[path_end..url_end] |
    /// description[url_end..description_end] | tags[description_end..tags_end]
    ///
    /// Tags are comma-separated within their slice.
    pub str_content: String,
    pub name_end: usize,
    pub path_end: usize,
    pub url_end: usize,
    pub description_end: usize,
    pub tags_end: usize,
    pub bpm: Option<u16>,
    pub sample_type: SampleType,
}

#[cfg(feature = "std")]
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
    /// Convenience constructor — builds the packed `str_content` from parts.
    pub fn new(
        name: &str,
        path: &str,
        url: &str,
        description: &str,
        tags: &[&str],
        bpm: Option<u16>,
        sample_type: SampleType,
    ) -> Self {
        let mut str_content = String::with_capacity(
            name.len()
                + path.len()
                + url.len()
                + description.len()
                + tags.iter().map(|t| t.len() + 1).sum::<usize>(),
        );

        str_content.push_str(name);
        let name_end = str_content.len();

        str_content.push_str(path);
        let path_end = str_content.len();

        str_content.push_str(url);
        let url_end = str_content.len();

        str_content.push_str(description);
        let description_end = str_content.len();

        for (i, tag) in tags.iter().enumerate() {
            if i > 0 {
                str_content.push(',');
            }
            str_content.push_str(tag);
        }
        let tags_end = str_content.len();

        Self {
            str_content,
            name_end,
            path_end,
            url_end,
            description_end,
            tags_end,
            bpm,
            sample_type,
        }
    }

    pub fn name(&self) -> &str {
        &self.str_content[..self.name_end]
    }
    pub fn path(&self) -> Option<&str> {
        let s = &self.str_content[self.name_end..self.path_end];
        if s.is_empty() { None } else { Some(s) }
    }
    pub fn url(&self) -> Option<&str> {
        let s = &self.str_content[self.path_end..self.url_end];
        if s.is_empty() { None } else { Some(s) }
    }
    pub fn description(&self) -> Option<&str> {
        let s = &self.str_content[self.url_end..self.description_end];
        if s.is_empty() { None } else { Some(s) }
    }
    pub fn tags(&self) -> impl Iterator<Item = &str> {
        self.str_content[self.description_end..self.tags_end].split(',')
    }

    pub fn clear_url(&mut self) {
        let url_len = self.url_end - self.path_end;

        if url_len == 0 {
            return;
        }

        self.str_content.drain(self.path_end..self.url_end);

        self.url_end -= url_len;
        self.description_end -= url_len;
        self.tags_end -= url_len;
    }

    /// Serialize this entry's fixed header + str_content into `buf`.
    fn write_into(&self, buf: &mut Vec<u8>) {
        let str_bytes = self.str_content.as_bytes();
        let str_len = str_bytes.len();

        buf.extend_from_slice(&(str_len as u32).to_le_bytes());
        buf.extend_from_slice(&(self.name_end as u32).to_le_bytes());
        buf.extend_from_slice(&(self.path_end as u32).to_le_bytes());
        buf.extend_from_slice(&(self.url_end as u32).to_le_bytes());
        buf.extend_from_slice(&(self.description_end as u32).to_le_bytes());
        buf.extend_from_slice(&(self.tags_end as u32).to_le_bytes());
        buf.extend_from_slice(&self.bpm.unwrap_or(0).to_le_bytes());
        buf.push(self.bpm.is_some() as u8);
        buf.push(self.sample_type.to_byte());
        buf.extend_from_slice(str_bytes);
    }
}

// -- frame I/O -----------------------------------------------------------------

/// Serialize a slice of `WireEntry` into a complete frame buffer.
pub fn write_frame(entries: &[WireEntry]) -> Vec<u8> {
    let capacity = FRAME_HEADER
        + entries
            .iter()
            .map(|e| REC_FIXED + e.str_content.len())
            .sum::<usize>();

    let mut buf = Vec::with_capacity(capacity);
    buf.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for entry in entries {
        entry.write_into(&mut buf);
    }
    buf
}

/// Parse a frame from a raw byte slice (starting at the count prefix).
/// Returns `(entries, bytes_consumed)`.
pub fn parse_frame(data: &[u8]) -> Result<(Vec<WireEntry>, usize), &'static str> {
    if data.len() < FRAME_HEADER {
        return Err("frame too short for count prefix");
    }

    let count = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
    let mut entries = Vec::with_capacity(count);
    let mut cur = FRAME_HEADER;

    for _ in 0..count {
        if cur + REC_FIXED > data.len() {
            return Err("record header out of bounds");
        }

        let h = &data[cur..];

        let str_len =
            u32::from_le_bytes(h[OFF_STR_LEN..OFF_STR_LEN + 4].try_into().unwrap()) as usize;
        let name_end =
            u32::from_le_bytes(h[OFF_NAME_END..OFF_NAME_END + 4].try_into().unwrap()) as usize;
        let path_end =
            u32::from_le_bytes(h[OFF_PATH_END..OFF_PATH_END + 4].try_into().unwrap()) as usize;
        let url_end =
            u32::from_le_bytes(h[OFF_URL_END..OFF_URL_END + 4].try_into().unwrap()) as usize;
        let description_end =
            u32::from_le_bytes(h[OFF_DESC_END..OFF_DESC_END + 4].try_into().unwrap()) as usize;
        let tags_end =
            u32::from_le_bytes(h[OFF_TAGS_END..OFF_TAGS_END + 4].try_into().unwrap()) as usize;
        let bpm_raw = u16::from_le_bytes(h[OFF_BPM..OFF_BPM + 2].try_into().unwrap());
        let has_bpm = h[OFF_HAS_BPM] != 0;
        let stype_byte = h[OFF_STYPE];

        if name_end > path_end
            || path_end > url_end
            || url_end > description_end
            || description_end > tags_end
            || tags_end > str_len
        {
            return Err("record has invalid index offsets");
        }

        let str_start = cur + REC_FIXED;
        let str_end = str_start + str_len;

        if str_end > data.len() {
            return Err("record str_content out of bounds");
        }

        let str_content = core::str::from_utf8(&data[str_start..str_end])
            .map_err(|_| "record str_content is not valid utf-8")?
            .into();

        let sample_type =
            SampleType::from_byte(stype_byte).ok_or("record has unknown sample_type byte")?;

        entries.push(WireEntry {
            str_content,
            name_end,
            path_end,
            url_end,
            description_end,
            tags_end,
            bpm: has_bpm.then_some(bpm_raw),
            sample_type,
        });

        cur = str_end;
    }

    Ok((entries, cur))
}
