//! Convenient imports for consumers of nes-core
//!
//! Pull in everything commonly needed in one line:
//! ```rust
//! use nes_core::prelude::*;
//! ```

// Main NES emulator API
pub use crate::nes::NES;
pub use crate::nes::cartridge::rom::{Rom, RomError};
pub use crate::nes::controller::joypad::JoypadButton;

// Traits that users might need
pub use crate::nes::cartridge::Cartridge;

// Macros
pub use crate::trace_dump;

// Constants
pub use crate::nes::ppu::consts::NES_SYSTEM_PALETTE;

// Conditional testing utilities
#[cfg(feature = "testing-utils")]
pub use crate::nes::test_utils::*;
