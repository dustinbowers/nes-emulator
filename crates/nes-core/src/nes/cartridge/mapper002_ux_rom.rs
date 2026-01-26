use super::Cartridge;
use super::rom::Mirroring;

#[derive(Debug)]
pub struct Mapper002UxRom {
    pub chr: Vec<u8>,
    pub chr_is_ram: bool,
    pub prg_rom: Vec<u8>,
    pub mirroring: Mirroring,
    bank_select: usize,
}

impl Mapper002UxRom {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: Mirroring) -> Mapper002UxRom {
        let chr_is_ram = chr_rom.len() == 0;
        Mapper002UxRom {
            prg_rom,
            chr: if chr_is_ram {
                vec![0u8; 0x2000]
            } else {
                chr_rom
            },
            mirroring,
            chr_is_ram,
            bank_select: 0,
        }
    }

    fn prg_bank_count(&self) -> usize {
        self.prg_rom.len() / 0x4000
    }
}

impl Cartridge for Mapper002UxRom {
    fn cpu_read(&mut self, addr: u16) -> u8 {
        let addr = addr as usize;
        if addr < self.chr.len() {
            self.chr[addr]
        } else {
            eprintln!("CHR read out of bounds: {:04X}", addr);
            0
        }
    }

    fn cpu_write(&mut self, addr: u16, data: u8) {
        if self.chr_is_ram {
            let addr = addr as usize % self.chr.len();
            self.chr[addr] = data;
        }
    }

    fn ppu_read(&mut self, addr: u16) -> u8 {
        let addr = addr as usize;
        let bank_size = 0x4000;
        let bank_count = self.prg_bank_count();

        match addr {
            0x8000..=0xBFFF => {
                // Switchable bank
                let base = self.bank_select % bank_count * bank_size;
                self.prg_rom[base + (addr - 0x8000)]
            }
            0xC000..=0xFFFF => {
                // Fixed bank
                let base = (bank_count - 1) * bank_size;
                self.prg_rom[base + (addr - 0xC000)]
            }
            _ => {
                eprintln!("PRG read out of bounds: {:04X}", addr);
                0
            }
        }
    }

    fn ppu_write(&mut self, addr: u16, data: u8) {
        /*
           7  bit  0
           ---- ----
           xxxx pPPP
                ||||
                ++++- Select 16 KB PRG ROM bank for CPU $8000-$BFFF
                     (UNROM uses bits 2-0; UOROM uses bits 3-0)
        */
        match addr {
            0x8000..=0xFFFF => {
                self.bank_select = (data & 0xF) as usize;
            }
            _ => {}
        }
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring.clone()
    }
}
