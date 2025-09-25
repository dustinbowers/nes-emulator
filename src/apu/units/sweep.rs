pub enum PulseType {
    Pulse1,
    Pulse2,
}

pub struct Sweep {
    pulse_type: PulseType,
    enabled: bool,
    reload: bool,
    period: u8,
    negate: bool,
    shift: u8,

    divider: u8, // counts down
    mute: bool,  // whether sweep would silence channel
}

impl Sweep {
    pub fn new(pulse_type: PulseType) -> Sweep {
        Sweep {
            pulse_type,
            enabled: false,
            reload: false,
            period: 0,
            negate: false,
            shift: 0,
            divider: 0,
            mute: false,
        }
    }

    pub fn set(&mut self, value: u8) {
        self.enabled = value & 0b1000_0000 != 0;
        self.period = (value >> 4) & 0b111;
        self.negate = value & 0b0000_1000 != 0;
        self.shift = value & 0b0000_0111;
        self.reload = true;
    }

    pub fn clock(&mut self, timer: &mut u16) {
        if self.divider == 0 {
            self.divider = self.period;
            if self.enabled && self.shift > 0 && !self.is_muting(*timer) {
                let change = *timer >> self.shift;
                let target = if self.negate {
                    match self.pulse_type {
                        PulseType::Pulse1 => *timer - change - 1, // ones’-complement
                        PulseType::Pulse2 => *timer - change,     // twos’-complement
                    }
                } else {
                    *timer + change
                };

                *timer = target;
            }
        } else {
            self.divider -= 1;
        }

        if self.reload {
            self.divider = self.period;
            self.reload = false;
        }
    }

    pub fn is_muting(&self, timer: u16) -> bool {
        if timer < 8 {
            return true;
        }
        let change = timer >> self.shift;
        let target = if self.negate {
            match self.pulse_type {
                PulseType::Pulse1 => timer - change - 1,
                PulseType::Pulse2 => timer - change,
            }
        } else {
            timer + change
        };
        target > 0x7FF // timer must fit in 11 bits
    }
}
