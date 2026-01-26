use rom::Mirroring;

pub mod mapper000_nrom;
pub mod mapper001_mmc1;
pub mod mapper002_ux_rom;
pub mod mapper003_cn_rom;
pub mod rom;
// mod mapper004_mmc3;

pub trait Cartridge {
    /// CPU read ($4020–$FFFF)
    fn cpu_read(&mut self, addr: u16) -> u8;

    /// CPU write ($4020–$FFFF)
    fn cpu_write(&mut self, addr: u16, data: u8);

    /// PPU read ($0000–$1FFF)
    fn ppu_read(&mut self, addr: u16) -> u8;

    /// PPU write ($0000–$1FFF)
    fn ppu_write(&mut self, addr: u16, data: u8);

    /// Nametable mirroring mode
    fn mirroring(&self) -> Mirroring;
}
