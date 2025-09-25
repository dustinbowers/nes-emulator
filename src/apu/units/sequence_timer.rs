pub struct SequenceTimer {
    reload_value: u16,
    value: u16,
}

impl SequenceTimer {
    pub fn new() -> SequenceTimer {
        SequenceTimer {
            reload_value: 0,
            value: 0,
        }
    }

    pub fn set_reload(&mut self, reload_value: u16) {
        self.reload_value = reload_value;
    }

    pub fn set_reload_high(&mut self, hi: u8) {
        self.reload_value = (self.reload_value & 0b1111_1111) | ((hi as u16) << 8);
    }

    pub fn set_reload_low(&mut self, lo: u8) {
        self.reload_value = (self.reload_value & 0b0111_0000_0000) | lo as u16;
    }

    /// returns `true` if waveform generator needs clocking
    pub fn clock(&mut self) -> bool {
        if self.value == 0 {
            self.value = self.reload_value;
            true
        } else {
            self.value -= 1;
            false
        }
    }

    pub fn reset(&mut self) {
        self.value = self.reload_value;
    }

    pub fn output(&self) -> u16 {
        return self.value;
    }
}
