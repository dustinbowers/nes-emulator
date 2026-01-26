use super::Cartridge;
use super::rom::Mirroring;

// MMC1 mapper (iNES mapper #1)
pub struct Mmc1 {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    chr_ram: Vec<u8>,

    // Shift register state
    shift_reg: u8,
    shift_count: u8,

    // Internal MMC1 registers
    control: u8,
    chr_bank0: u8,
    chr_bank1: u8,
    prg_bank: u8,

    prg_ram: Vec<u8>,
}

impl Mmc1 {
    pub fn new(prg_rom: Vec<u8>, chr_data: Vec<u8>, prg_ram_size: usize) -> Self {
        let has_chr_ram = chr_data.is_empty();
        Mmc1 {
            prg_rom,
            chr_rom: if has_chr_ram {
                vec![0; 0x2000]
            } else {
                chr_data
            },
            chr_ram: if has_chr_ram {
                vec![0; 0x2000]
            } else {
                Vec::new()
            },
            shift_reg: 0x10,
            shift_count: 0,
            control: 0x0C, // default: PRG mode=3, CHR mode=0, nametable=0
            chr_bank0: 0,
            chr_bank1: 0,
            prg_bank: 0,
            prg_ram: vec![0; prg_ram_size],
        }
    }

    // Helper to update MMC1 shift register
    fn mmc1_write(&mut self, addr: u16, data: u8) {
        // Reset shift register if bit 7 set ($80-$FF)
        if data & 0x80 != 0 {
            self.shift_reg = 0x10;
            self.shift_count = 0;
            self.control = (self.control & !0x0C) | 0x0C; // set PRG mode = 3
            return;
        }

        // Shift in one bit (LSB first)
        let bit = data & 1;
        self.shift_reg = (self.shift_reg >> 1) | (bit << 4);
        self.shift_count += 1;

        if self.shift_count == 5 {
            // Determine which register to update
            match addr {
                0x8000..=0x9FFF => self.control = self.shift_reg & 0x1F,    // 5 bits
                0xA000..=0xBFFF => self.chr_bank0 = self.shift_reg & 0x1F,  // 5 bits
                0xC000..=0xDFFF => self.chr_bank1 = self.shift_reg & 0x1F,  // 5 bits
                0xE000..=0xFFFF => self.prg_bank = self.shift_reg & 0x0F,   // 4 bits
                _ => unreachable!()

            }
            // Reset for next series of writes
            self.shift_reg = 0x10;
            self.shift_count = 0;
        }
    }

    fn ppu_bank_addr(&self, addr: u16) -> u16 {
        let mode_4k = self.control & 0x10 != 0;
        let bank_addr = if mode_4k {
            let bank_sel = if addr < 0x1000 {
                self.chr_bank0
            } else {
                self.chr_bank1
            };
            let max_banks = (self.chr_rom.len() / 0x1000).max(1) as u8;
            let bank_sel = bank_sel % max_banks;
            (bank_sel as u16) * 0x1000 + (addr & 0x0FFF)
        } else {
            let bank_sel = (self.chr_bank0 & 0x1E) as u16;
            bank_sel * 0x2000 + addr
        };
        bank_addr
    }
}

impl Cartridge for Mmc1 {
    fn cpu_read(&mut self, addr: u16) -> (u8, bool) {
        let addr = addr as usize;
        match addr {
            0x6000..=0x7FFF => {
                // 8KB PRG-RAM bank (optional)
                let prg_ram_enabled = self.prg_bank & 0x10 == 0;
                if prg_ram_enabled {
                    let idx = addr - 0x6000;
                    let data = self.prg_ram[idx % 0x2000]; // Mirror if above 8KB
                    (data, false)
                } else {
                    (0, true) // open bus
                }
            }
            0x8000..=0xFFFF => {
                let addr = (addr - 0x8000) as usize;
                let prg_size = self.prg_rom.len();
                let mode = (self.control >> 2) & 0b11;

                let bank_addr = match mode {
                    0 | 1 => {
                        let bank = (self.prg_bank & 0x0E) as usize;
                        let base = bank * 0x4000; // 16KB units
                        (base + addr) % prg_size
                    }
                    2 => {
                        if addr < 0x4000 {
                            addr
                        } else {
                            let bank_sel = (self.prg_bank & 0x0F) as usize;
                            (bank_sel * 0x4000 + (addr - 0x4000)) % prg_size
                        }
                    }
                    3 => {
                        if addr < 0x4000 {
                            let bank_sel = (self.prg_bank & 0x0F) as usize;
                            (bank_sel * 0x4000 + addr) % prg_size
                        } else {
                            let last = prg_size - 0x4000;
                            last + (addr - 0x4000)
                        }
                    }
                    _ => unreachable!(),
                };

                (self.prg_rom[bank_addr % prg_size], false)

            }
            _ => (0, true) // open-bus
        }
    }

    fn cpu_write(&mut self, addr: u16, data: u8) {
        // Note: MMC1 requires one CPU cycle between writes on real hardware
        //       This shouldn't be a problem given my CPU is memory-cycle accurate
        match addr {
            0x6000..=0x7FFF => {
                // write PRG RAM if enabled
                let prg_ram_enabled = self.prg_bank & 0x10 == 0;
                if prg_ram_enabled {
                    let offset = (addr - 0x6000) as usize;
                    if offset < self.prg_ram.len() {
                        self.prg_ram[offset] = data;
                    }
                }
            }
            0x8000..=0xFFFF => {
                // Mapper register writes
                self.mmc1_write(addr, data);
            }
            _ => {}
        }
    }

    fn ppu_read(&mut self, addr: u16) -> (u8, bool) {
        let bank_addr = self.ppu_bank_addr(addr) as usize;
        let mut data = 0;
        if !self.chr_ram.is_empty() {
            data =self.chr_ram[bank_addr % self.chr_ram.len()];
        } else {
            data = self.chr_rom[bank_addr % self.chr_rom.len()];
        }
        (data, false)
    }

    fn ppu_write(&mut self, addr: u16, data: u8) {
        if !self.chr_ram.is_empty() {
            let bank_addr = self.ppu_bank_addr(addr) as usize;
            let chr_ram_len = self.chr_ram.len();
            self.chr_ram[bank_addr % chr_ram_len] = data;
        }
    }

    fn mirroring(&self) -> Mirroring {
        match self.control & 0x03 {
            0 => Mirroring::Single0,
            1 => Mirroring::Single1,
            2 => Mirroring::Vertical,
            3 => Mirroring::Horizontal,
            _ => unreachable!(),
        }
    }
}
