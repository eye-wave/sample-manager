#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
