#[cfg(feature = "std")]
use std::sync::Arc;

#[cfg(feature = "std")]
use serde::Serialize;
#[cfg(feature = "std")]
use ts_rs::TS;

#[cfg(not(feature = "std"))]
use alloc::sync::Arc;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use crate::SampleType;

#[cfg_attr(feature = "std", derive(Debug, Clone, Serialize, TS))]
pub struct SampleMetadata {
    pub tags: Vec<Arc<str>>,
    pub description: Option<Arc<str>>,
    pub bpm: Option<u16>,
    #[cfg_attr(feature = "std", serde(rename = "type"))]
    pub sample_type: SampleType,
}
