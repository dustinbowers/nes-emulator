pub struct LengthCounter {
    halted: bool,
    value: u8,
}

impl LengthCounter {
    pub fn new() -> LengthCounter {
        LengthCounter {
            halted: false,
            value: 0,
        }
    }

    pub fn set_halt(&mut self, halted: bool) {
        self.halted = halted;
        if self.halted {
            self.value = 0;
        }
    }

    pub fn set(&mut self, pos: u8) {
        if self.halted {
            return;
        }
        const COUNT_LOOKUP: [u8; 32] = [
            10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20,
            96, 22, 192, 24, 72, 26, 16, 28, 32, 30,
        ];
        self.value = COUNT_LOOKUP[(pos & 0b1_1111) as usize];
    }

    pub fn clock(&mut self) -> bool {
        if self.halted == false && self.value > 0 {
            self.value -= 1;
        }

        // match self.value {
        //     0 => false,
        //     _ => true,
        // }
        match self.value {
            0 => true,
            _ => false,
        }
    }

    pub fn output(&self) -> u8 {
        return self.value;
    }
}
