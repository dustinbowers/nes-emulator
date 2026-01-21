use super::Cartridge;
use crate::nes::cartridge::rom::Mirroring;

#[derive(Debug)]
pub struct NromCart {
    pub chr: Vec<u8>,
    pub chr_is_ram: bool,
    pub prg_rom: Vec<u8>,
    pub mirroring: Mirroring,
}

impl NromCart {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: Mirroring) -> NromCart {
        let chr_is_ram = chr_rom.len() == 0;
        NromCart {
            prg_rom,
            chr: if chr_is_ram {
                vec![0u8; 0x2000]
            } else {
                chr_rom
            },
            mirroring,
            chr_is_ram,
        }
    }
}

impl Cartridge for NromCart {
    fn chr_read(&mut self, addr: u16) -> u8 {
        let addr = addr as usize;
        if addr < self.chr.len() {
            self.chr[addr]
        } else {
            eprintln!("CHR read out of bounds: {:04X}", addr);
            0
        }
    }

    fn chr_write(&mut self, addr: u16, data: u8) {
        if self.chr_is_ram {
            let addr = addr as usize % self.chr.len();
            self.chr[addr] = data;
        }
    }

    fn prg_read(&mut self, addr: u16) -> u8 {
        let addr = addr as usize - 0x8000;

        let addr = if self.prg_rom.len() == 0x4000 {
            addr % 0x4000 // mirror if only 16KB PRG
        } else {
            addr
        };

        let b = self.prg_rom[addr];
        b
    }

    fn prg_write(&mut self, _: u16, _data: u8) {
        // NOP
    }

    fn mirroring(&self) -> Mirroring {
        // TODO: avoid .clone() here
        self.mirroring.clone()
    }
}
