pub struct SequenceTimer {
    timer_low: u8,
    timer_high: u8,
    reload_value: u16,
    value: u16,
}

impl SequenceTimer {
    pub fn new() -> SequenceTimer {
        SequenceTimer {
            timer_low: 0,
            timer_high: 0,
            reload_value: 0, // 11-bits in hardware
            value: 0,
        }
    }

    pub fn set_reload(&mut self, reload_value: u16) {
        // Mask to 11-bits
        self.reload_value = reload_value & 0x07FF;
    }

    /// Called with value from writes to $400A
    pub fn set_reload_low(&mut self, lo: u8) {
        self.timer_low = lo;
        self.reload_value = (self.timer_high as u16) << 8 | (self.timer_low as u16);
        self.reload_value &= 0b0111_1111_1111;
    }

    /// Called with value from writes to $400B (only low 3 bits are used)
    pub fn set_reload_high(&mut self, hi: u8) {
        self.timer_high = hi & 0b111;
        self.reload_value = (self.timer_high as u16) << 8 | (self.timer_low as u16);
        self.reload_value &= 0b0111_1111_1111;
    }

    /// Returns `true` if waveform generator needs clocking
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timer_ticks_after_reload_plus_one_clocks() {
        let mut t = SequenceTimer::new();
        t.set_reload(3);
        t.reset(); // start from reload

        // For reload=3, the event should occur every 4 clocks
        // Clock 1: 3->2 (no)
        assert!(!t.clock());
        // Clock 2: 2->1 (no)
        assert!(!t.clock());
        // Clock 3: 1->0 (no)
        assert!(!t.clock());
        // Clock 4: 0 reloads to 3, event fires
        assert!(t.clock());
        assert_eq!(t.output(), 3);
    }

    #[test]
    fn timer_event_when_starting_at_zero() {
        let mut t = SequenceTimer::new();
        t.set_reload(5);

        // value starts at 0, next clock is true and timer reloads
        assert!(t.clock());
        assert_eq!(t.output(), 5);
    }

    #[test]
    fn set_reload_low_high_builds_11bit_value() {
        let mut t = SequenceTimer::new();

        // hi=0b111 (7), lo=0xAA => 0x7AA
        t.set_reload_low(0xAA);
        t.set_reload_high(0b111);
        assert_eq!(t.get_reload(), 0x7AA);

        // hi should be masked to 3 bits:
        t.set_reload_high(0xFF); // -> 0b111
        assert_eq!(t.get_reload(), 0x7AA);
    }

    #[test]
    fn masking_keeps_reload_11bit() {
        let mut t = SequenceTimer::new();
        // ensure reload masks down to 11 bits
        t.set_reload(0xFFFF);
        assert_eq!(t.get_reload(), 0x7FF);
    }
}
