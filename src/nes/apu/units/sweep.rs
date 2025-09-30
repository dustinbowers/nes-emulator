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
        // println!("sweep::set({:08b})", value);
        self.enabled = value & 0b1000_0000 != 0;
        self.period = (value >> 4) & 0b111;
        self.negate = value & 0b0000_1000 != 0;
        self.shift = value & 0b0000_0111;
        self.reload = true;
    }

    pub fn clock(&mut self, timer: &mut u16) {
        let mut apply_change = false;

        if self.divider == 0 {
            if self.enabled && self.shift > 0 && !self.is_muting(*timer) {
                apply_change = true;
            }
        }

        if apply_change {
            let change = *timer >> self.shift;
            let target = if self.negate {
                match self.pulse_type {
                    PulseType::Pulse1 => {
                        if change > *timer - 1 {
                            0
                        } else {
                            *timer - change - 1
                        }
                        // (*timer).wrapping_sub(change).wrapping_sub(1)
                    }
                    PulseType::Pulse2 => {
                        if change > *timer {
                            0
                        } else {
                            *timer - change
                        }
                        // (*timer).wrapping_sub(change)
                    }
                }
            } else {
                *timer + change
            };
            *timer = target;
        }

        if self.divider == 0 || self.reload {
            self.divider = self.period;
            self.reload = false;
        } else {
            self.divider -= 1;
        }
    }

    pub fn is_muting(&self, timer: u16) -> bool {
        if timer < 8 {
            return true;
        }
        // if self.divider < 8 {
        //     return true;
        // }
        // if self.period < 8 {
        //     return true;
        // }
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

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    pub fn set_period(&mut self, period: u8) {
        self.period = period;
    }
    pub fn get_period(&self) -> u8 {
        self.period
    }
    pub fn set_shift(&mut self, shift: u8) {
        self.shift = shift;
    }
    pub fn get_shift(&self) -> u8 {
        self.shift
    }
    pub fn get_negate_flag(&self) -> bool {
        self.negate
    }
}
