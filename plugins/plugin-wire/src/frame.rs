use crate::types::SampleType;
#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

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
//   [+12.. +16]         u32  description_end
//   [+16.. +20]         u32  tags_end
//   [+20.. +22]         u16  bpm  (0 when has_bpm = 0)
//   [+22]               u8   has_bpm  (0 or 1)
//   [+23]               u8   sample_type
//   [+24.. +24+str_len] u8[str_len]  str_content utf-8
//
// str_content packs: name | path | description | comma-separated tags
// The four _end offsets are byte positions within str_content.

const OFF_STR_LEN: usize = 0;
const OFF_NAME_END: usize = 4;
const OFF_PATH_END: usize = 8;
const OFF_DESC_END: usize = 12;
const OFF_TAGS_END: usize = 16;
const OFF_BPM: usize = 20;
const OFF_HAS_BPM: usize = 22;
const OFF_STYPE: usize = 23;
const REC_FIXED: usize = 24;
const FRAME_HEADER: usize = 4; // u32 record_count

// -- WireEntry — the in-memory representation of one record --------------------

/// One sample entry in the wire format, ready to be written or just parsed.
///
/// `str_content` is the packed string: `name || path || description || tags`
/// where `||` means concatenation. The four `*_end` fields are byte offsets
/// into `str_content` marking where each section ends.
#[derive(Debug, Clone)]
pub struct WireEntry {
    /// name[0..name_end] | path[name_end..path_end] |
    /// description[path_end..description_end] | tags[description_end..tags_end]
    ///
    /// Tags are comma-separated within their slice.
    pub str_content: String,
    pub name_end: usize,
    pub path_end: usize,
    pub description_end: usize,
    pub tags_end: usize,
    pub bpm: Option<u16>,
    pub sample_type: SampleType,
}

impl WireEntry {
    /// Convenience constructor — builds the packed `str_content` from parts.
    pub fn new(
        name: &str,
        path: &str,
        description: &str,
        tags: &[&str],
        bpm: Option<u16>,
        sample_type: SampleType,
    ) -> Self {
        let mut str_content = String::with_capacity(
            name.len()
                + path.len()
                + description.len()
                + tags.iter().map(|t| t.len() + 1).sum::<usize>(),
        );

        str_content.push_str(name);
        let name_end = str_content.len();

        str_content.push_str(path);
        let path_end = str_content.len();

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
            description_end,
            tags_end,
            bpm,
            sample_type,
        }
    }

    pub fn name(&self) -> &str {
        &self.str_content[..self.name_end]
    }
    pub fn path(&self) -> &str {
        &self.str_content[self.name_end..self.path_end]
    }
    pub fn description(&self) -> &str {
        &self.str_content[self.path_end..self.description_end]
    }
    pub fn tags(&self) -> impl Iterator<Item = &str> {
        self.str_content[self.description_end..self.tags_end].split(',')
    }

    /// Serialize this entry's fixed header + str_content into `buf`.
    fn write_into(&self, buf: &mut Vec<u8>) {
        let str_bytes = self.str_content.as_bytes();
        let str_len = str_bytes.len();

        buf.extend_from_slice(&(str_len as u32).to_le_bytes());
        buf.extend_from_slice(&(self.name_end as u32).to_le_bytes());
        buf.extend_from_slice(&(self.path_end as u32).to_le_bytes());
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
///
/// Returns the owned `Vec<u8>` — the plugin side calls this then passes the
/// pointer back to the host via `mem::write_frame_ptr`.
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
///
/// `bytes_consumed` is the total number of bytes that make up this frame —
/// the host passes this back to `free(frame_ptr, bytes_consumed)`.
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
        let description_end =
            u32::from_le_bytes(h[OFF_DESC_END..OFF_DESC_END + 4].try_into().unwrap()) as usize;
        let tags_end =
            u32::from_le_bytes(h[OFF_TAGS_END..OFF_TAGS_END + 4].try_into().unwrap()) as usize;
        let bpm_raw = u16::from_le_bytes(h[OFF_BPM..OFF_BPM + 2].try_into().unwrap());
        let has_bpm = h[OFF_HAS_BPM] != 0;
        let stype_byte = h[OFF_STYPE];

        if name_end > path_end
            || path_end > description_end
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
            description_end,
            tags_end,
            bpm: has_bpm.then_some(bpm_raw),
            sample_type,
        });

        cur = str_end;
    }

    Ok((entries, cur))
}
