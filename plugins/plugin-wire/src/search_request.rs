#[cfg(not(feature = "std"))]
use alloc::string::String;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

/// wire layout (LE, host -> plugin):
///   [0..4]   u32  limit
///   [4..8]   u32  offset
///   [8]      u8   is_fav  (0 = false, 1 = true)
///   [9..12]  u8[3] _padding
///   [12..16] u32  query_len
///   [16..]   u8[query_len]  query utf-8
///
/// Tags are handled by the host - plugins never see them.
#[derive(Debug)]
#[repr(C)]
pub struct SearchRequestWire {
    pub limit: usize,
    pub offset: usize,
    pub is_fav: bool,
    pub query: String,
}

/// Decode a SearchRequest from the binary wire format.
/// Returns `None` if the buffer is too short or the query is not valid UTF-8.
pub fn decode_search_request(bytes: &[u8]) -> Option<SearchRequestWire> {
    if bytes.len() < 16 {
        return None;
    }

    let limit = u32::from_le_bytes(bytes[0..4].try_into().ok()?) as usize;
    let offset = u32::from_le_bytes(bytes[4..8].try_into().ok()?) as usize;
    let is_fav = bytes[8] != 0;
    // bytes[9..12] = padding, intentionally ignored
    let query_len = u32::from_le_bytes(bytes[12..16].try_into().ok()?) as usize;

    if bytes.len() < 16 + query_len {
        return None;
    }

    let query = core::str::from_utf8(&bytes[16..16 + query_len])
        .ok()?
        .into();

    Some(SearchRequestWire {
        limit,
        offset,
        is_fav,
        query,
    })
}

/// Encode a SearchRequest into the binary wire format.
/// Used by the host before writing the buffer into wasm memory.
pub fn encode_search_request(limit: usize, offset: usize, is_fav: bool, query: &str) -> Vec<u8> {
    let query_bytes = query.as_bytes();

    let mut buf = Vec::with_capacity(16 + query_bytes.len());
    buf.extend_from_slice(&(limit as u32).to_le_bytes());
    buf.extend_from_slice(&(offset as u32).to_le_bytes());
    buf.push(is_fav as u8);
    buf.extend_from_slice(&[0u8; 3]); // padding - keeps query_len 4-byte aligned
    buf.extend_from_slice(&(query_bytes.len() as u32).to_le_bytes());
    buf.extend_from_slice(query_bytes);
    buf
}
