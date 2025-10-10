pub struct DmcOutput {
    register: u8,
    bits_remaining: u8,
    current_bit: u8,
    silence_flag: bool,
    
    level: u8,
}

impl DmcOutput {
    pub fn new() -> DmcOutput {
        DmcOutput {
            register: 0,
            bits_remaining: 0,
            current_bit: 0,
            silence_flag: false,
            
            level: 0,
        }
    }

    pub fn set_register(&mut self, value: u8) {
        self.current_bit = 8;
        self.register = value;
    }
    
    pub fn direct_load(&mut self, value: u8) {
        self.level = value;
    }
}

impl DmcOutput {
    pub fn clock(&mut self) {
        
    }

    pub fn output(&self) -> u8 {
        self.level
    }
}
