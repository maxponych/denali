#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Fifo,
    CharDevice,
    Directory,
    BlockDevice,
    Regular,
    Symlink,
    Socket,
    Cell,
    Unknown,
}

impl FileType {
    pub fn from_mode(mode: u32) -> Self {
        match mode & 0xF000 {
            0x1000 => FileType::Fifo,
            0x2000 => FileType::CharDevice,
            0x4000 => FileType::Directory,
            0x6000 => FileType::BlockDevice,
            0x8000 => FileType::Regular,
            0xA000 => FileType::Symlink,
            0xC000 => FileType::Socket,
            0xB000 => FileType::Cell,
            _ => FileType::Unknown,
        }
    }
}
