#![feature(const_index)]
#![feature(const_trait_impl)]

//! Zero-allocation trie runner.
//!
//! # Binary layout (little-endian)
//!
//! ```text
//! [dict_size: u32][dict_data: ...]
//! [tree_size: u32][tree_data: ...]
//! ```
//!
//! ## NodeChunk (inside tree_data)
//! ```text
//! [+0..+2]  u16  header      (bit15=strict | bits14..10=num_outputs | bit9=delta_flag | bits8..0=num_connections)
//! [+2..]    N × 2 bytes  connection  (u8 slot, i8 delta)      — if delta_flag=1
//!        or N × 3 bytes  connection  (u8 slot, u16 absolute)  — if delta_flag=0
//! [+2+N*s]  M × 3 bytes  output      (u16 start LE, u8 len)   — only present if NOUT > 0
//! ```

#![no_std]

#[inline]
fn read_u8(buf: &[u8], offset: usize) -> u8 {
    buf[offset]
}

#[inline]
fn read_u16_le(buf: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([buf[offset], buf[offset + 1]])
}

#[inline]
const fn read_u32_le(buf: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
    ])
}

/// Decode the UTF-8 codepoint that *ends* at byte index `end` (exclusive)
/// within `buf`, i.e. the last codepoint before position `end`.
/// Returns `None` if `end == 0`.
#[inline]
fn prev_char(buf: &[u8], end: usize) -> Option<char> {
    if end == 0 {
        return None;
    }
    // Walk back up to 4 bytes to find the start of the codepoint.
    let start = end.saturating_sub(4);
    let slice = &buf[start..end];
    // The last valid char in this slice is the one ending at `end`.
    core::str::from_utf8(slice).ok()?.chars().next_back()
}

/// Decode the UTF-8 codepoint that *starts* at byte index `start` within `buf`.
/// Returns `None` if `start >= buf.len()`.
#[inline]
fn next_char(buf: &[u8], start: usize) -> Option<char> {
    if start >= buf.len() {
        return None;
    }
    // At most 4 bytes needed for one codepoint.
    let end = (start + 4).min(buf.len());
    core::str::from_utf8(&buf[start..end])
        .ok()
        .and_then(|s| s.chars().next())
}

/// A character is a word character if the charset can encode it —
/// i.e. it belongs to one of the supported scripts.
#[inline]
fn is_word_char(ch: char) -> bool {
    tagger_charset::encode_normalised(ch).is_some()
}

// Model

#[derive(Debug, PartialEq)]
pub struct Model<'a> {
    dict: &'a [u8],
    tree: &'a [u8],
}

impl<'a> Model<'a> {
    pub const fn from_bytes(data: &'a [u8]) -> Self {
        if data.len() < 4 {
            panic!("LoadError::TooShort");
        }

        let dict_size = read_u32_le(data, 0) as usize;
        let dict_end = 4 + dict_size;
        if dict_end > data.len() {
            panic!("LoadError::DictChunkOutOfBounds");
        }
        let dict = &data[4..dict_end];

        let tree_header = dict_end;
        if tree_header + 4 > data.len() {
            panic!("LoadError::TooShort");
        }
        let tree_size = read_u32_le(data, tree_header) as usize;
        let tree_start = tree_header + 4;
        let tree_end = tree_start + tree_size;
        if tree_end > data.len() {
            panic!("LoadError::TreeChunkOutOfBounds");
        }
        let tree = &data[tree_start..tree_end];

        Self { dict, tree }
    }

    #[inline]
    fn node_header(&self, node_off: usize) -> u16 {
        read_u16_le(self.tree, node_off)
    }

    #[inline]
    fn node_is_strict(&self, node_off: usize) -> bool {
        self.node_header(node_off) & 0x8000 != 0
    }

    #[inline]
    fn node_is_delta(&self, node_off: usize) -> bool {
        self.node_header(node_off) & 0x0200 != 0
    }

    #[inline]
    fn node_num_connections(&self, node_off: usize) -> usize {
        (self.node_header(node_off) & 0x01FF) as usize
    }

    #[inline]
    fn node_num_outputs(&self, node_off: usize) -> usize {
        ((self.node_header(node_off) >> 10) & 0x1F) as usize
    }

    #[inline]
    fn node_follow(&self, node_off: usize, slot: u8) -> Option<usize> {
        let n = self.node_num_connections(node_off);
        let is_delta = self.node_is_delta(node_off);
        let base = node_off + 2;
        if is_delta {
            for i in 0..n {
                let entry = base + i * 2;
                if read_u8(self.tree, entry) == slot {
                    let delta = read_u8(self.tree, entry + 1) as i8;
                    return Some((node_off as isize + delta as isize) as usize);
                }
            }
        } else {
            for i in 0..n {
                let entry = base + i * 3;
                if read_u8(self.tree, entry) == slot {
                    return Some(read_u16_le(self.tree, entry + 1) as usize);
                }
            }
        }
        None
    }

    #[inline]
    fn node_outputs_offset(&self, node_off: usize) -> usize {
        let stride = if self.node_is_delta(node_off) { 2 } else { 3 };
        node_off + 2 + self.node_num_connections(node_off) * stride
    }

    #[inline]
    fn node_output_str(&self, node_off: usize, i: usize) -> &'a str {
        let base = self.node_outputs_offset(node_off) + i * 3;
        let start = read_u16_le(self.tree, base) as usize;
        let len = read_u8(self.tree, base + 2) as usize;
        core::str::from_utf8(&self.dict[start..start + len]).unwrap_or("")
    }

    /// Search `text` for all matching tags, calling `f` once per unique tag found.
    ///
    /// Slides a window over every character position in `text`. At each start
    /// position it walks the trie character by character. Whenever a node with
    /// outputs is reached, the surrounding characters (just before the start and
    /// just after the current position) are checked: if the node is strict, both
    /// must be non-alphabetic boundaries; otherwise outputs are always emitted.
    ///
    /// Duplicate tags are suppressed — `f` is called at most once per unique tag.
    pub fn search<F>(&self, text: &str, mut f: F)
    where
        F: FnMut(&'a str),
    {
        if self.tree.is_empty() {
            return;
        }

        let chars: &[u8] = text.as_bytes();
        let len = chars.len();

        const MAX_TAGS: usize = 64;
        let mut seen: [&str; MAX_TAGS] = [""; MAX_TAGS];
        let mut seen_count = 0usize;

        let mut i = 0usize;
        while i < len {
            let mut node_off = 0usize;
            let mut j = i;

            while j < len {
                let slot = match tagger_charset::encode(chars[j] as char) {
                    Some(s) => s,
                    None => break,
                };

                match self.node_follow(node_off, slot) {
                    Some(next) => node_off = next,
                    None => break,
                }

                let m = self.node_num_outputs(node_off);
                if m > 0 {
                    let strict = self.node_is_strict(node_off);

                    if strict {
                        let left_ok = prev_char(chars, i).is_none_or(|c| !is_word_char(c));
                        let right_ok = next_char(chars, j + 1).is_none_or(|c| !is_word_char(c));
                        if !left_ok || !right_ok {
                            j += 1;
                            continue;
                        }
                    }

                    for k in 0..m {
                        let tag = self.node_output_str(node_off, k);
                        let already = seen[..seen_count]
                            .iter()
                            .any(|&s: &&str| s.as_ptr() == tag.as_ptr() && s.len() == tag.len());
                        if !already {
                            if seen_count < MAX_TAGS {
                                seen[seen_count] = tag;
                                seen_count += 1;
                            }
                            f(tag);
                        }
                    }
                }

                j += 1;
            }

            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use std::vec::Vec;
    use tagger_compiler::compile;

    fn model_from_src(src: &str) -> Vec<u8> {
        compile(src).expect("compile failed")
    }

    fn search_all<'a>(model: &'a Model<'a>, text: &str) -> Vec<&'a str> {
        let mut tags = Vec::new();
        model.search(text, |t| tags.push(t));
        tags
    }

    #[test]
    fn single_word_with_named_output() {
        let bytes = model_from_src("cat *noun\n");
        let model = Model::from_bytes(&bytes);
        assert_eq!(search_all(&model, "cat"), ["noun"]);
    }

    #[test]
    fn word_itself_output() {
        let bytes = model_from_src("cat *\n");
        let model = Model::from_bytes(&bytes);
        assert_eq!(search_all(&model, "cat"), ["cat"]);
    }

    #[test]
    fn no_match_returns_empty() {
        let bytes = model_from_src("cat *noun\n");
        let model = Model::from_bytes(&bytes);
        assert!(search_all(&model, "dog").is_empty());
    }

    #[test]
    fn prefix_does_not_match() {
        let bytes = model_from_src("cat *noun\n");
        let model = Model::from_bytes(&bytes);
        assert!(search_all(&model, "ca").is_empty());
    }

    #[test]
    fn group_output_inherited() {
        let src = "animals {\ncat\ndog\n}\n";
        let bytes = model_from_src(src);
        let model = Model::from_bytes(&bytes);
        assert_eq!(search_all(&model, "cat"), ["animals"]);
        assert_eq!(search_all(&model, "dog"), ["animals"]);
    }

    #[test]
    fn word_overrides_group_with_extra_output() {
        let src = "animals {\ncat *feline\n}\n";
        let bytes = model_from_src(src);
        let model = Model::from_bytes(&bytes);
        let mut tags = search_all(&model, "cat");
        tags.sort();
        assert_eq!(tags, ["animals", "feline"]);
    }

    #[test]
    fn strict_matches_whole_word() {
        let bytes = model_from_src("!men *noun\n");
        let model = Model::from_bytes(&bytes);
        assert_eq!(search_all(&model, "men"), ["noun"]);
        assert!(search_all(&model, "documents").is_empty());
        assert!(search_all(&model, "the men went").contains(&"noun"));
    }

    #[test]
    fn non_strict_matches_substring() {
        let bytes = model_from_src("men *noun\n");
        let model = Model::from_bytes(&bytes);
        assert!(!search_all(&model, "documents").is_empty());
    }

    #[test]
    fn optional_char_matches_both_forms() {
        let bytes = model_from_src("pre# *tag\n");
        let model = Model::from_bytes(&bytes);
        assert_eq!(search_all(&model, "pre"), ["tag"]);
        assert_eq!(search_all(&model, "pre_"), ["tag"]);
    }

    #[test]
    fn repeating_char_matches_multiple() {
        let bytes = model_from_src("as+ *tag\n");
        let model = Model::from_bytes(&bytes);
        assert_eq!(search_all(&model, "as"), ["tag"]);
        assert_eq!(search_all(&model, "ass"), ["tag"]);
        assert_eq!(search_all(&model, "asss"), ["tag"]);
    }

    #[test]
    fn multiple_named_outputs() {
        let bytes = model_from_src("cat *noun *animal\n");
        let model = Model::from_bytes(&bytes);
        let mut tags = search_all(&model, "cat");
        tags.sort();
        assert_eq!(tags, ["animal", "noun"]);
    }

    #[test]
    fn wildcard_edge_case() {
        let src = "bass {\nre+se *reese\nres *res\n}";
        let bytes = model_from_src(src);
        let model = Model::from_bytes(&bytes);
        let mut reese = search_all(&model, "reese");
        reese.sort();
        assert_eq!(reese, ["bass", "reese"]);
        let mut res = search_all(&model, "res");
        res.sort();
        assert_eq!(res, ["bass", "res"]);
    }
}
