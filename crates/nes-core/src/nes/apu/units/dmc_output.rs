pub struct DmcOutput {
    shift_register: u8,
    level: u8,
    bits_remaining: u8,
}

impl DmcOutput {
    pub fn new() -> DmcOutput {
        DmcOutput {
            shift_register: 0,
            level: 0,
            bits_remaining: 0,
        }
    }

    /// Load DMC shift register with a new sample byte
    /// Resets the bit counter to 8
    pub fn load_shift_register(&mut self, value: u8) {
        self.shift_register = value;
        self.bits_remaining = 8;
    }

    /// $4011 direct load (7-bit)
    pub fn direct_load(&mut self, value: u8) {
        self.level = value & 0x7F;
    }

    /// Clock dmc output unit once (at DMC rate)
    /// Returns `true` if shift register is empty after this clock
    pub fn clock(&mut self) -> bool {
        if self.bits_remaining == 0 {
            // nothing remaining to output
            return true;
        }

        let bit = self.shift_register & 1;
        self.shift_register >>= 1;
        self.bits_remaining -= 1;

        // Add delta to level if its in range
        if bit == 0 && self.level <= 125 {
            self.level += 2;
        } else if bit == 1 && self.level >= 2 {
            self.level -= 2;
        }

        // Let DMC know when we're out of bits
        self.bits_remaining == 0
    }

    pub fn level(&self) -> u8 {
        self.level
    }
}
