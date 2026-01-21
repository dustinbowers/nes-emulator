pub struct SequenceTimer {
    timer_low: u8,
    timer_high: u8,
    pub reload_value: u16,
    value: u16,
}

impl SequenceTimer {
    pub fn new() -> SequenceTimer {
        SequenceTimer {
            timer_low: 0,
            timer_high: 0,
            reload_value: 0,
            value: 0,
        }
    }

    pub fn set_reload(&mut self, reload_value: u16) {
        self.reload_value = reload_value;
    }

    pub fn set_reload_low(&mut self, lo: u8) {
        self.timer_low = lo;
        self.reload_value = (self.timer_high as u16) << 8 | (self.timer_low as u16);
        self.reload_value &= 0b0111_1111_1111;
    }

    pub fn set_reload_high(&mut self, hi: u8) {
        self.timer_high = hi & 0b111;
        self.reload_value = (self.timer_high as u16) << 8 | (self.timer_low as u16);
        self.reload_value &= 0b0111_1111_1111;
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
        self.value
    }

    pub fn get_reload(&self) -> u16 {
        self.reload_value
    }
    pub fn get_reload_low_bits(&self) -> u8 {
        self.timer_low
    }
    pub fn get_reload_high_bits(&self) -> u8 {
        self.timer_high
    }
}
