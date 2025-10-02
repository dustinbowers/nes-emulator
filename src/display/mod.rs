use std::fmt::{Display, Formatter};
use crate::nes::cartridge::rom::RomError;

pub mod color_map;
pub mod consts;
pub mod shared_frame;

impl Display for RomError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RomError::InvalidFormat(msg) => write!(f, "{}", msg),
            RomError::UnsupportedVersion(msg) => write!(f, "{}", msg),
            // RomError::OutOfBounds(msg) => write!(f, "{}", msg),
        }
    }
}
