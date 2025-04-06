use crate::rom::Mirroring;
use super::Cartridge;

pub struct NromCart {
    pub chr_rom: Vec<u8>,
    pub prg_rom: Vec<u8>,
    pub mirroring: Mirroring,
}

impl NromCart {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: Mirroring) -> NromCart {
        NromCart {
            prg_rom,
            chr_rom,
            mirroring,
        }
    }
}

impl Cartridge for NromCart {
    fn chr_read(&mut self, addr: u16) -> u8 {
        let addr = addr % self.chr_rom.len() as u16; // wrap when out of bounds
        self.chr_rom[addr as usize]
    }

    fn chr_write(&mut self, addr: u16, data: u8) {
        self.chr_rom[addr as usize] = data;
    }

    fn prg_read(&mut self, addr: u16) -> u8 {
        let addr = addr as usize - 0x8000;

        let addr = if self.prg_rom.len() == 0x4000 {
            addr % 0x4000 // mirror if only 16KB PRG
        } else {
            addr
        };

        self.prg_rom[addr]
    }

    fn prg_write(&mut self, _: u16, _data: u8) {
        // NOP
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring.clone()
    }

    fn get_prg_rom(&self) -> Vec<u8> {
        self.prg_rom.clone()
    }
}