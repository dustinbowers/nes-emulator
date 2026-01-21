use rom::Mirroring;

pub mod mapper000_nrom;
pub mod mapper001_mmc1;
pub mod mapper002_ux_rom;
pub mod mapper003_cn_rom;
pub mod rom;

pub trait Cartridge {
    /// Read a byte from CHR space ($0000–$1FFF)
    fn chr_read(&mut self, addr: u16) -> u8;
    /// Write a byte into CHR space (only for CHR‑RAM carts)
    fn chr_write(&mut self, addr: u16, data: u8);

    /// Read a byte from PRG space ($8000–$FFFF)
    fn prg_read(&mut self, addr: u16) -> u8;
    /// Write a byte into PRG space (for mappers with PRG‑RAM)
    fn prg_write(&mut self, addr: u16, data: u8);

    /// Get the current mirroring mode
    fn mirroring(&self) -> Mirroring;
}
