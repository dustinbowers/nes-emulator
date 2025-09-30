use super::Cartridge;
use super::rom::Mirroring;

#[derive(Debug)]
pub struct Mapper003CnRom {
    pub chr: Vec<u8>,
    pub prg_rom: Vec<u8>,
    pub mirroring: Mirroring,
    bank_select: usize,
}

impl Mapper003CnRom {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: Mirroring) -> Mapper003CnRom {
        Mapper003CnRom {
            prg_rom,
            chr: if chr_rom.is_empty() {
                vec![0u8; 0x2000] // CHR RAM fallback (rare for CNROM)
            } else {
                chr_rom
            },
            mirroring,
            bank_select: 0,
        }
    }

    fn prg_bank_count(&self) -> usize {
        self.prg_rom.len() / 0x4000
    }

    fn chr_bank_count(&self) -> usize {
        self.chr.len() / 0x2000
    }
}

impl Cartridge for Mapper003CnRom {
    fn chr_read(&mut self, addr: u16) -> u8 {
        let addr = addr as usize;
        let bank_size = 0x2000;
        let bank_count = self.chr_bank_count();
        let bank = self.bank_select % bank_count;
        let base = bank * bank_size;

        if addr < bank_size {
            self.chr[base + addr]
        } else {
            eprintln!("CHR read out of bounds: {:04X}", addr);
            0
        }
    }

    fn chr_write(&mut self, addr: u16, data: u8) {
        // Only valid if using CHR RAM (rare for CNROM)
        let bank_size = 0x2000;
        if self.chr_bank_count() == 0 {
            let addr = addr as usize % bank_size;
            self.chr[addr] = data;
        }
    }

    fn prg_read(&mut self, addr: u16) -> u8 {
        let addr = addr as usize;
        let prg_size = self.prg_rom.len();

        match addr {
            0x8000..=0xFFFF => {
                let mapped = (addr - 0x8000) % prg_size;
                self.prg_rom[mapped]
            }
            _ => {
                eprintln!("PRG read out of bounds: {:04X}", addr);
                0
            }
        }
    }

    fn prg_write(&mut self, addr: u16, data: u8) {
        /*
           7  bit  0
           ---- ----
           xxxx CCCC
                ||||
                ++++- Select 8 KB CHR ROM bank at PPU $0000
                       (CNROM uses bits 2-0; sometimes 3-0)
        */
        match addr {
            0x8000..=0xFFFF => {
                self.bank_select = (data & 0x03) as usize; // usually 2 bits, sometimes 4
            }
            _ => {}
        }
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring.clone()
    }
}
