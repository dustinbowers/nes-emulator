use super::Cartridge;
use super::rom::Mirroring;

pub struct Mmc3 {
    prg_rom: Vec<u8>,
    chr: Vec<u8>,
    chr_is_ram: bool,
    prg_ram: Vec<u8>,

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

    // Note: A12 refers to the 12th bit of the NES's 14-bit address bus
    // The PPU crosses between nametables regularly ($0xxx and $1xxx)
    // and this creates a regular rising edge on A12 that signals new scanlines.
    // It's effectively a clever way of clocking scanlines without
    // having a specific way to do it
    last_ppu_a12: bool,
    a12_low_cycles: u8,

    mirroring: Mirroring,
    mirroring_fixed: bool,
    prg_ram_enabled: bool,
    prg_ram_write_protect: bool,
    irq_pending: bool,
}

impl Mmc3 {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        let chr_is_ram = chr_rom.is_empty();
        let chr = if chr_is_ram {
            vec![0u8; 0x2000]
        } else {
            chr_rom
        };

        let prg_banks = (prg_rom.len() / 0x2000).max(1);
        let chr_banks = (chr.len() / 0x0400).max(1);

        let mirroring_fixed = matches!(mirroring, Mirroring::FourScreen);

        Self {
            prg_rom,
            chr,
            chr_is_ram,
            prg_ram: vec![0u8; 0x2000],
            prg_banks,
            chr_banks,
            bank_select: 0,
            bank_registers: [0; 8],
            prg_mode: false,
            chr_mode: false,
            irq_latch: 0,
            irq_counter: 0,
            irq_reload: false,
            irq_enabled: false,
            last_ppu_a12: false,
            a12_low_cycles: 0,
            mirroring,
            mirroring_fixed,
            prg_ram_enabled: true,
            prg_ram_write_protect: false,
            irq_pending: false,
        }
    }

    fn prg_bank_index(&self, bank: usize) -> usize {
        bank % self.prg_banks
    }

    fn chr_bank_index(&self, bank: usize) -> usize {
        bank % self.chr_banks
    }

    fn prg_addr(&self, addr: u16) -> usize {
        let offset = (addr as usize) & 0x1FFF;
        let last = self.prg_banks.saturating_sub(1);
        let second_last = self.prg_banks.saturating_sub(2);

        let bank = match addr {
            0x8000..=0x9FFF => {
                if self.prg_mode {
                    second_last
                } else {
                    self.bank_registers[6] as usize
                }
            }
            0xA000..=0xBFFF => self.bank_registers[7] as usize,
            0xC000..=0xDFFF => {
                if self.prg_mode {
                    self.bank_registers[6] as usize
                } else {
                    second_last
                }
            }
            0xE000..=0xFFFF => last,
            _ => 0,
        };

        self.prg_bank_index(bank) * 0x2000 + offset
    }

    fn chr_addr(&self, addr: u16) -> usize {
        let offset = (addr as usize) & 0x03FF;
        let bank = match addr {
            0x0000..=0x03FF => {
                if self.chr_mode {
                    self.bank_registers[2] as usize
                } else {
                    (self.bank_registers[0] & 0xFE) as usize
                }
            }
            0x0400..=0x07FF => {
                if self.chr_mode {
                    self.bank_registers[3] as usize
                } else {
                    (self.bank_registers[0] | 1) as usize
                }
            }
            0x0800..=0x0BFF => {
                if self.chr_mode {
                    self.bank_registers[4] as usize
                } else {
                    (self.bank_registers[1] & 0xFE) as usize
                }
            }
            0x0C00..=0x0FFF => {
                if self.chr_mode {
                    self.bank_registers[5] as usize
                } else {
                    (self.bank_registers[1] | 1) as usize
                }
            }
            0x1000..=0x13FF => {
                if self.chr_mode {
                    (self.bank_registers[0] & 0xFE) as usize
                } else {
                    self.bank_registers[2] as usize
                }
            }
            0x1400..=0x17FF => {
                if self.chr_mode {
                    (self.bank_registers[0] | 1) as usize
                } else {
                    self.bank_registers[3] as usize
                }
            }
            0x1800..=0x1BFF => {
                if self.chr_mode {
                    (self.bank_registers[1] & 0xFE) as usize
                } else {
                    self.bank_registers[4] as usize
                }
            }
            0x1C00..=0x1FFF => {
                if self.chr_mode {
                    (self.bank_registers[1] | 1) as usize
                } else {
                    self.bank_registers[5] as usize
                }
            }
            _ => 0,
        };

        self.chr_bank_index(bank) * 0x0400 + offset
    }

    fn clock_irq(&mut self, addr: u16) {
        let a12 = (addr & 0x1000) != 0;

        if !a12 {
            // Track how long A12 stays low
            self.a12_low_cycles = self.a12_low_cycles.saturating_add(1);
        } else {
            // Rising edge?
            if !self.last_ppu_a12 && self.a12_low_cycles >= 8 {
                if self.irq_reload || self.irq_counter == 0 {
                    self.irq_counter = self.irq_latch;
                } else {
                    self.irq_counter = self.irq_counter.wrapping_sub(1);

                    if self.irq_counter == 0 && self.irq_enabled {
                        self.irq_pending = true;
                    }
                }

                self.irq_reload = false;
            }

            self.a12_low_cycles = 0;
        }

        self.last_ppu_a12 = a12;
    }
}

impl Cartridge for Mmc3 {
    fn cpu_read(&mut self, addr: u16) -> (u8, bool) {
        match addr {
            0x6000..=0x7FFF => {
                if self.prg_ram_enabled && !self.prg_ram.is_empty() {
                    let i = (addr - 0x6000) as usize;
                    (self.prg_ram[i % 0x2000], false)
                } else {
                    (0, true)
                }
            }
            0x8000..=0xFFFF => {
                let i = self.prg_addr(addr);
                (self.prg_rom[i % self.prg_rom.len()], false)
            }
            _ => (0, true),
        }
    }

    fn cpu_write(&mut self, addr: u16, data: u8) {
        match addr {
            0x6000..=0x7FFF => {
                if self.prg_ram_enabled && !self.prg_ram_write_protect {
                    let i = (addr - 0x6000) as usize;
                    if !self.prg_ram.is_empty() {
                        self.prg_ram[i % 0x2000] = data;
                    }
                }
            }
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
                    if !self.mirroring_fixed {
                        self.mirroring = if data & 1 == 0 {
                            Mirroring::Vertical
                        } else {
                            Mirroring::Horizontal
                        };
                    }
                } else {
                    self.prg_ram_enabled = data & 0x80 != 0;
                    self.prg_ram_write_protect = data & 0x40 != 0;
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
                    self.irq_pending = false;
                } else {
                    self.irq_enabled = true;
                    // self.irq_reload = true;
                }
            }
            _ => {}
        }
    }

    fn ppu_read(&mut self, addr: u16) -> (u8, bool) {
        let i = self.chr_addr(addr);
        (self.chr[i % self.chr.len()], false)
    }

    fn ppu_write(&mut self, addr: u16, data: u8) {
        if self.chr_is_ram {
            let i = self.chr_addr(addr);
            let chr_len = self.chr.len();
            self.chr[i % chr_len] = data;
        }
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring
    }

    fn irq_pending(&self) -> bool {
        self.irq_pending
    }

    fn ppu_clock(&mut self, addr: u16) {
        self.clock_irq(addr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a12_low(mmc3: &mut Mmc3, cycles: u8) {
        for _ in 0..cycles {
            mmc3.clock_irq(0x0FFF); // A12 = 0
        }
    }

    fn a12_rise(mmc3: &mut Mmc3) {
        mmc3.clock_irq(0x1000); // A12 = 1
    }

    #[test]
    fn mmc3_decrements_on_a12_rise() {
        let mut mmc3 = Mmc3::new(vec![0; 0x8000], vec![0; 0x2000], Mirroring::Vertical);

        mmc3.cpu_write(0xC000, 5); // latch = 5
        mmc3.cpu_write(0xC001, 0); // reload
        mmc3.cpu_write(0xE001, 0); // enable IRQ

        // First clock → reload only
        a12_low(&mut mmc3, 8);
        a12_rise(&mut mmc3);
        assert_eq!(mmc3.irq_counter, 5);

        // Second clock → decrement
        a12_low(&mut mmc3, 8);
        a12_rise(&mut mmc3);
        assert_eq!(mmc3.irq_counter, 4);
    }

    #[test]
    fn mmc3_c000_does_not_reload() {
        let mut mmc3 = Mmc3::new(vec![0; 0x8000], vec![0; 0x2000], Mirroring::Vertical);

        mmc3.irq_counter = 3;
        mmc3.cpu_write(0xC000, 7);

        assert_eq!(mmc3.irq_counter, 3);
    }

    #[test]
    fn mmc3_c001_sets_reload_flag() {
        let mut mmc3 = Mmc3::new(vec![0; 0x8000], vec![0; 0x2000], Mirroring::Vertical);

        mmc3.irq_counter = 4;
        mmc3.cpu_write(0xC001, 0);

        // Not reloaded yet
        assert_eq!(mmc3.irq_counter, 4);

        // Next clock reloads
        a12_low(&mut mmc3, 8);
        a12_rise(&mut mmc3);
        assert_eq!(mmc3.irq_counter, mmc3.irq_latch);
    }

    #[test]
    fn mmc3_irq_fires_on_decrement_to_zero() {
        let mut mmc3 = Mmc3::new(vec![0; 0x8000], vec![0; 0x2000], Mirroring::Vertical);

        mmc3.cpu_write(0xC000, 1);
        mmc3.cpu_write(0xC001, 0);
        mmc3.cpu_write(0xE001, 0);

        // Reload
        a12_low(&mut mmc3, 8);
        a12_rise(&mut mmc3);

        // Decrement to zero
        a12_low(&mut mmc3, 8);
        a12_rise(&mut mmc3);

        assert!(mmc3.irq_pending());
    }

    #[test]
    fn mmc3_irq_does_not_fire_when_disabled() {
        let mut mmc3 = Mmc3::new(vec![0; 0x8000], vec![0; 0x2000], Mirroring::Vertical);

        mmc3.cpu_write(0xC000, 1);
        mmc3.cpu_write(0xC001, 0);
        mmc3.cpu_write(0xE000, 0); // disable

        for _ in 0..4 {
            a12_low(&mut mmc3, 8);
            a12_rise(&mut mmc3);
        }

        assert!(!mmc3.irq_pending());
    }

    #[test]
    fn mmc3_reload_when_counter_zero() {
        let mut mmc3 = Mmc3::new(vec![0; 0x8000], vec![0; 0x2000], Mirroring::Vertical);

        mmc3.cpu_write(0xC000, 3);
        mmc3.cpu_write(0xC001, 0);

        mmc3.irq_counter = 0;

        a12_low(&mut mmc3, 8);
        a12_rise(&mut mmc3);

        assert_eq!(mmc3.irq_counter, 3);
    }

    #[test]
    fn mmc3_ignores_short_a12_pulses() {
        let mut mmc3 = Mmc3::new(vec![0; 0x8000], vec![0; 0x2000], Mirroring::Vertical);

        mmc3.cpu_write(0xC000, 2);
        mmc3.cpu_write(0xC001, 0);

        a12_low(&mut mmc3, 1); // too short
        a12_rise(&mut mmc3);

        assert_eq!(mmc3.irq_counter, 0);
    }
}
