pub struct LengthCounter {
    enabled: bool,
    halted: bool,
    value: u8,
}

/* Source: nes-test-roms/apu_test/source/2-len_table.s
   table:  .byte 10, 254, 20,  2, 40,  4, 80,  6
           .byte 160,  8, 60, 10, 14, 12, 26, 14
           .byte 12,  16, 24, 18, 48, 20, 96, 22
           .byte 192, 24, 72, 26, 16, 28, 32, 30
*/
#[rustfmt::skip]
const COUNT_LOOKUP: [u8; 32] = [
    10, 254, 20,  2, 40,  4, 80,  6,
    160,  8, 60, 10, 14, 12, 26, 14,
    12,  16, 24, 18, 48, 20, 96, 22,
    192, 24, 72, 26, 16, 28, 32, 30,
];

impl LengthCounter {
    pub fn new() -> LengthCounter {
        LengthCounter {
            enabled: false,
            halted: false,
            value: 0,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if self.enabled == false {
            self.value = 0;
        }
    }

    pub fn set_halt(&mut self, halted: bool) {
        self.halted = halted;
    }

    pub fn load_index(&mut self, index: u8) {
        // ignore load when not enabled
        if !self.enabled {
            return;
        }
        self.value = COUNT_LOOKUP[(index & 0b1_1111) as usize];
    }

    pub fn clock(&mut self) {
        if self.value > 0 && !self.halted {
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

    #[test]
    fn initial_state() {
        let lc = LengthCounter::new();
        assert_eq!(lc.halted, false);
        assert_eq!(lc.value, 0);
    }

    #[test]
    fn set_value() {
        let mut lc = LengthCounter::new();
        lc.set_enabled(true);
        lc.load_index(5);
        assert_eq!(lc.value, 4);
    }

    #[test]
    fn clock_to_zero() {
        let mut lc = LengthCounter::new();
        lc.set_enabled(true);
        lc.load_index(5); // position 5 in lookup table sets counter to 4
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

    #[test]
    fn write_ignored_when_disabled() {
        let mut lc = LengthCounter::new();
        lc.set_enabled(false); // sets length to 0
        lc.load_index(5); // write ignored if !enabled
        assert_eq!(lc.output(), 0); // length still 0
    }

    #[test]
    fn disabling_clears_counter_immediate() {
        let mut lc = LengthCounter::new();
        lc.set_enabled(true);
        lc.load_index(5); // COUNT_LOOKUP[5] => 4
        assert_eq!(lc.output(), 4);

        lc.set_enabled(false); // sets length to 0
        assert_eq!(lc.output(), 0);
        lc.set_enabled(true); // re-enable doesn't restore previous
        assert_eq!(lc.output(), 0);
    }

    #[test]
    fn halt_prevents_decrements() {
        let mut lc = LengthCounter::new();
        lc.set_enabled(true);
        lc.load_index(5); // index 5 => 4
        lc.set_halt(true);

        for _ in 0..10 {
            lc.clock();
        }
        assert_eq!(lc.output(), 4);
    }

    #[test]
    fn clock_decrements_to_zero_and_stops() {
        let mut lc = LengthCounter::new();
        lc.set_enabled(true);
        lc.load_index(5); // 4

        lc.set_halt(false);
        assert_eq!(lc.output(), 4);
        lc.clock();
        assert_eq!(lc.output(), 3);
        lc.clock();
        assert_eq!(lc.output(), 2);
        lc.clock();
        assert_eq!(lc.output(), 1);
        lc.clock();
        assert_eq!(lc.output(), 0);

        for _ in 0..10 {
            lc.clock();
        }
        assert_eq!(lc.output(), 0); // still zero
    }

    #[test]
    fn table_matches_known_entries() {
        let mut lc = LengthCounter::new();
        lc.set_enabled(true);

        // Check a few entries
        lc.load_index(0);
        assert_eq!(lc.output(), 10);
        lc.load_index(1);
        assert_eq!(lc.output(), 254);
        lc.load_index(2);
        assert_eq!(lc.output(), 20);
        lc.load_index(31);
        assert_eq!(lc.output(), 30);
    }

    #[test]
    fn index_is_masked_to_5_bits() {
        let mut lc = LengthCounter::new();
        lc.set_enabled(true);

        lc.load_index(0x20); // masked to 0
        assert_eq!(lc.output(), 10);

        lc.load_index(0x3F); // masked to 31
        assert_eq!(lc.output(), 30);
    }
}
