pub struct LengthCounter {
    enabled: bool,
    halted: bool,
    value: u8,
}

impl LengthCounter {
    pub fn new() -> LengthCounter {
        LengthCounter {
            enabled: false,
            halted: false,
            value: 0,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if self.enabled == false {
            self.value = 0;
            self.halted = true;
        }
    }

    pub fn set_halt(&mut self, halted: bool) {
        self.halted = halted;
    }

    pub fn set(&mut self, pos: u8) {
        if self.halted {
            // Don't change value if counter is halted
            return;
        }
        const COUNT_LOOKUP: [u8; 32] = [
            10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20,
            96, 22, 192, 24, 72, 26, 16, 28, 32, 30,
        ];
        self.value = COUNT_LOOKUP[(pos & 0b1_1111) as usize];
        self.enabled = self.value > 0;
    }

    pub fn clock(&mut self) {
        if !self.halted && self.value > 0 {
            self.value -= 1;
        }
    }

    pub fn output(&self) -> u8 {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn setup_length_counter() -> LengthCounter {
        LengthCounter::new()
    }

    #[test]
    fn test_initial_state() {
        let lc = setup_length_counter();
        assert_eq!(lc.halted, false);
        assert_eq!(lc.value, 0);
    }

    #[test]
    fn test_set_value() {
        let mut lc = setup_length_counter();
        lc.set(5);
        assert_eq!(lc.value, 4);
    }

    #[test]
    fn test_clock_to_zero() {
        let mut lc = setup_length_counter();
        lc.set(5); // position 5 in lookup table sets counter to 4
        assert_eq!(lc.value, 4);
        lc.clock();
        assert_eq!(lc.value, 3);
        lc.clock();
        assert_eq!(lc.value, 2);
        lc.clock();
        assert_eq!(lc.value, 1);
        lc.clock();
        assert_eq!(lc.value, 0);
        lc.clock();
        assert_eq!(lc.value, 0);
    }
}
