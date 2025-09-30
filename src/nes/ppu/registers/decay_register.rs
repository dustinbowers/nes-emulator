pub struct DecayRegister {
    value: u8,
    addr: u16,
    period: usize,
    cycle: usize,
}

impl DecayRegister {
    pub fn new(period: usize) -> DecayRegister {
        DecayRegister {
            value: 0,
            addr: 0,
            period,
            cycle: period,
        }
    }

    pub fn tick(&mut self) {
        self.cycle -= 1;
        if self.cycle <= 0 {
            println!("CLEARED DECAY REGISTER");
            self.value = 0;
        }
    }

    pub fn set(&mut self, addr: u16, value: u8) {
        self.addr = addr & 0x3FFF;
        self.value = value;
        self.cycle = self.period;
    }

    pub fn output(&self) -> u8 {
        return self.value;
    }
}
