use super::rom::Mirroring;
use super::Cartridge;

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
        if data & 0x80 != 0 {
            // Reset shift register
            self.shift_reg = 0x10;
            self.shift_count = 0;
            self.control |= 0x0C; // set PRG mode = 3
            return;
        }

        // Shift in one bit (LSB first)
        let bit = data & 1;
        self.shift_reg = (self.shift_reg >> 1) | (bit << 4);
        self.shift_count += 1;

        if self.shift_count == 5 {
            // Determine which register to update
            let reg = (addr >> 13) & 0x03; // 0:$8000,1:$A000,2:$C000,3:$E000
            match reg {
                0 => self.control = self.shift_reg,
                1 => self.chr_bank0 = self.shift_reg,
                2 => self.chr_bank1 = self.shift_reg,
                3 => self.prg_bank = self.shift_reg & 0x0F,
                _ => unreachable!(),
            }
            // Reset for next series of writes
            self.shift_reg = 0x10;
            self.shift_count = 0;
        }
    }
}

impl Cartridge for Mmc1 {
    fn chr_read(&mut self, addr: u16) -> u8 {
        let mode_4k = self.control & 0x10 != 0;
        let addr = addr as usize;
        let bank = if mode_4k {
            // 4KB mode
            let bank_sel = if addr < 0x1000 {
                self.chr_bank0
            } else {
                self.chr_bank1
            };
            (bank_sel as usize) * 0x1000 + (addr & 0x0FFF)
        } else {
            // 8KB mode
            let bank_sel = (self.chr_bank0 & 0x0E) as usize;
            bank_sel * 0x1000 + addr
        };
        self.chr_rom[bank]
    }

    fn chr_write(&mut self, addr: u16, data: u8) {
        if !self.chr_ram.is_empty() {
            let addr = addr as usize % 0x2000;
            self.chr_ram[addr] = data;
        }
        // else: CHR-ROM, ignore writes
    }

    fn prg_read(&mut self, addr: u16) -> u8 {
        let addr = addr as usize - 0x8000;
        let prg_size = self.prg_rom.len();
        let bank_mode = (self.control >> 2) & 0x03;
        let bank = match bank_mode {
            0 | 1 => {
                // 32KB switch, ignore low bit of prg_bank
                let bank_sel = (self.prg_bank & 0x0E) as usize;
                (bank_sel * 0x4000 + addr) % prg_size
            }
            2 => {
                // Fix first bank at $8000, switch at $C000
                if addr < 0x4000 {
                    addr
                } else {
                    let bank_sel = self.prg_bank as usize;
                    bank_sel * 0x4000 + (addr - 0x4000)
                }
            }
            3 => {
                // Switch at $8000, fix last bank at $C000
                let last_bank = prg_size - 0x4000;
                if addr < 0x4000 {
                    let bank_sel = self.prg_bank as usize;
                    bank_sel * 0x4000 + addr
                } else {
                    last_bank + (addr - 0x4000)
                }
            }
            _ => unreachable!(),
        };
        self.prg_rom[bank]
    }

    fn prg_write(&mut self, addr: u16, data: u8) {
        match addr {
            0x6000..=0x7FFF => {
                // PRG RAM (if any)
                let offset = (addr - 0x6000) as usize;
                if offset < self.prg_ram.len() {
                    self.prg_ram[offset] = data;
                }
            }
            0x8000..=0xFFFF => {
                // Mapper register writes
                self.mmc1_write(addr, data);
            }
            _ => {}
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
