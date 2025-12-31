pub enum PackType {
    Project = 0x01,
    Snapshot = 0x02,
    Object = 0x03,
    Main = 0x04,
    NotFound = 0xFF,
}

impl PackType {
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x01 => Some(Self::Project),
            0x02 => Some(Self::Snapshot),
            0x03 => Some(Self::Object),
            0x04 => Some(Self::Main),
            0xFF => Some(Self::NotFound),
            _ => None,
        }
    }

    pub fn as_byte(&self) -> u8 {
        match self {
            PackType::Project => 0x01,
            PackType::Snapshot => 0x02,
            PackType::Object => 0x03,
            PackType::Main => 0x04,
            PackType::NotFound => 0xFF,
        }
    }
}
