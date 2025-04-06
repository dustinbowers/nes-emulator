use crate::rom::Mirroring;

pub mod nrom;

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

    /// Get a clone of PRG_ROM
    fn get_prg_rom(&self) -> Vec<u8>;
}