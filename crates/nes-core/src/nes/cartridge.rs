use rom::Mirroring;

pub mod mapper000_nrom;
pub mod mapper001_mmc1;
pub mod mapper002_ux_rom;
pub mod mapper003_cn_rom;
pub mod mapper004_mmc3;
pub mod rom;
// mod mapper004_mmc3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapperTiming {
    None,
    Mmc1,
}

pub trait Cartridge: Send {
    /// CPU read ($4020–$FFFF)
    ///
    /// # Returns
    ///
    /// A `(u8, bool)` tuple containing the data as `u8` and open bus as `bool`
    fn cpu_read(&mut self, addr: u16) -> (u8, bool);

    /// CPU write ($4020–$FFFF)
    fn cpu_write(&mut self, addr: u16, data: u8);

    /// PPU read ($0000–$1FFF)
    ///
    /// # Returns
    ///
    /// A `(u8, bool)` tuple containing the data as `u8` and open bus as `bool`
    fn ppu_read(&mut self, addr: u16) -> (u8, bool);

    /// PPU write ($0000–$1FFF)
    fn ppu_write(&mut self, addr: u16, data: u8);

    /// Nametable mirroring mode
    fn mirroring(&self) -> Mirroring;

    /// Bus-visible timing quirks
    fn timing(&self) -> MapperTiming {
        MapperTiming::None
    }

    fn irq_pending(&self) -> bool {
        false
    }

    fn ppu_clock(&mut self, addr: u16) {

    }
}
