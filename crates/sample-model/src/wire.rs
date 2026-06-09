use serde::{Deserialize, Serialize};
use ts_rs::TS;

mod entry;
mod search_request;

pub use entry::*;
pub use search_request::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[repr(u8)]
pub enum SampleType {
    OneShot = 0,
    Loop = 1,
}

impl SampleType {
    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            0 => Some(Self::OneShot),
            1 => Some(Self::Loop),
            _ => None,
        }
    }

    pub fn to_byte(self) -> u8 {
        self as u8
    }
}
