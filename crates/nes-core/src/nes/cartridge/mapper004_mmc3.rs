use crate::nes::cartridge::rom::Mirroring;
use crate::prelude::Cartridge;

pub struct Mmc3 {
    prg_rom: Vec<u8>,
    chr: Vec<u8>,
    chr_is_ram: bool,

    prg_banks: usize,
    chr_banks: usize,

    bank_select: u8,
    bank_registers: [u8; 8],

    prg_mode: bool,
    chr_mode: bool,

    irq_latch: u8,
    irq_counter: u8,
    irq_reload: bool,
    irq_enabled: bool,

    last_ppu_a12: bool,

    mirroring: Mirroring
}

impl Cartridge for Mmc3 {
    fn cpu_read(&mut self, addr: u16) -> u8 {
        match addr {
            0x8000..=0xFFFF => {
                let i = self.prg_addr(addr);
                self.prg_rom[i]
            }
            _ => 0,
        }
    }

    fn cpu_write(&mut self, addr: u16, data: u8) {
        match addr {
            0x8000..=0x9FFF => {
                if addr & 1 == 0 {
                    self.bank_select = data;
                    self.prg_mode = data & 0x40 != 0;
                    self.chr_mode = data & 0x80 != 0;
                } else {
                    let r = (self.bank_select & 0x07) as usize;
                    self.bank_registers[r] = data;
                }
            }
            0xA000..=0xBFFF => {
                if addr & 1 == 0 {
                    self.mirroring = if data & 1 == 0 {
                        Mirroring::Vertical
                    } else {
                        Mirroring::Horizontal
                    };
                }
            }
            0xC000..=0xDFFF => {
                if addr & 1 == 0 {
                    self.irq_latch = data;
                } else {
                    self.irq_reload = true;
                }
            }
            0xE000..=0xFFFF => {
                if addr & 1 == 0 {
                    self.irq_enabled = false;
                } else {
                    self.irq_enabled = true;
                }
            }
            _ => {}
        }
    }

    fn ppu_read(&mut self, addr: u16) -> u8 {
        self.clock_irq(addr);

        let i = self.chr_bank(addr);
        self.chr[i]
    }

    fn ppu_write(&mut self, addr: u16, data: u8) {
        if self.chr_is_ram {
            let i = self.chr_bank(addr);
            self.chr[i] = data;
        }
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring
    }
}
