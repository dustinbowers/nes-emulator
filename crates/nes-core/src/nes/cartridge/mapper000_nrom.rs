use super::Cartridge;
use crate::nes::cartridge::rom::Mirroring;

#[derive(Debug)]
pub struct NromCart {
    pub chr: Vec<u8>,
    pub chr_is_ram: bool,
    pub prg_rom: Vec<u8>,
    pub prg_ram: Vec<u8>,
    pub mirroring: Mirroring,
}

impl NromCart {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: Mirroring) -> NromCart {
        let chr_is_ram = chr_rom.is_empty();
        NromCart {
            prg_rom,
            prg_ram: vec![0; 0x2000],
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
    fn cpu_read(&mut self, addr: u16) -> (u8, bool) {
        match addr {
            0x6000..=0x7FFF => {
                let index = (addr - 0x6000) as usize;
                (self.prg_ram[index], false)
            }
            0x8000..=0xFFFF => {
                let mut index = addr - 0x8000;
                if self.prg_rom.len() == 0x4000 {
                    index %= 0x4000; // mirror 16kb
                }
                (self.prg_rom[index as usize], false)
            }
            _ => (0, true),
        }
    }

    fn cpu_write(&mut self, addr: u16, data: u8) {
        if let 0x6000..=0x7FFF = addr {
            let index = (addr - 0x6000) as usize;
            self.prg_ram[index] = data;
        }
    }

    fn ppu_read(&mut self, addr: u16) -> (u8, bool) {
        let addr = addr as usize;
        if addr < self.chr.len() {
            (self.chr[addr], false)
        } else {
            // TODO: this may not be exceptional?
            panic!("CHR read out of bounds: {:04X}", addr);
            // (0, true)
        }
    }

    fn ppu_write(&mut self, addr: u16, data: u8) {
        if self.chr_is_ram {
            let addr = addr as usize % self.chr.len();
            self.chr[addr] = data;
        }
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring
    }
}
