use serde::{Deserialize, Serialize};

use crate::SearchRequest;

#[derive(Debug, Deserialize, Serialize)]
pub struct SearchRequestWire<'a> {
    pub limit: usize,
    pub offset: usize,
    pub is_fav: bool,
    pub query: &'a str,
}

impl<'a> SearchRequestWire<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, postcard::Error> {
        postcard::from_bytes(bytes)
    }
}

impl SearchRequest {
    pub fn to_bytes(&self) -> Result<Vec<u8>, postcard::Error> {
        let wire: SearchRequestWire = self.into();
        postcard::to_allocvec(&wire)
    }
}

impl<'a> From<&'a SearchRequest> for SearchRequestWire<'a> {
    fn from(value: &'a SearchRequest) -> Self {
        Self {
            limit: value.limit,
            offset: value.offset,
            is_fav: value.is_fav,
            query: &value.query,
        }
    }
}
