pub struct DmcOutput {
    register: u8,
    bits_remaining: u8,
    current_bit: u8,
    silence_flag: bool,
}

impl DmcOutput {
    pub fn new() -> DmcOutput {
        DmcOutput {
            register: 0,
            bits_remaining: 0,
            current_bit: 0,
            silence_flag: false,
        }
    }

    pub fn set_register(&mut self, value: u8) {
        self.register = value;
    }
}

impl DmcOutput {
    pub fn clock(&mut self) {}

    pub fn output() -> u8 {
        // TODO
        0
    }
}
