pub struct Envelope {
    start: bool,
    divider: u8,
    decay: u8,
    constant_volume: u8,
    loop_flag: bool,
    disable: bool,
    period: u8,
}

impl Envelope {
    pub fn new() -> Envelope {
        Envelope {
            start: false,
            divider: 0,
            decay: 0,
            constant_volume: 0,
            loop_flag: false,
            disable: false,
            period: 0,
        }
    }

    pub fn start(&mut self) {
        self.start = false;
        self.decay = 15;
        self.divider = self.period;
    }

    pub fn set(&mut self, value: u8) {
        self.period = value & 0b0000_1111; // lower 4 bits
        self.constant_volume = self.period;
        self.loop_flag = (value & 0b0010_0000) != 0; // bit 5
        self.disable = (value & 0b0001_0000) != 0; // bit 4
    }

    /// Called by the quarter-frame clock
    pub fn clock(&mut self) {
        if self.start {
            self.start = false;
            self.decay = 15;
            self.divider = self.period;
        } else {
            if self.divider == 0 {
                self.divider = self.period;
                if self.decay > 0 {
                    self.decay -= 1;
                } else if self.loop_flag {
                    self.decay = 15;
                }
            } else {
                self.divider -= 1;
            }
        }
    }

    pub fn output(&self) -> u8 {
        if self.disable {
            self.constant_volume
        } else {
            self.decay
        }
    }
}
