pub struct Registers {
    data: u32,
}
impl Registers {
    pub fn new() -> Registers {
        Registers { data: 0 }
    }

    pub fn write(&mut self, reg: usize, value: u8) {
        match reg {
            0 => {
                let mask: u32 = 0xFF << 24;
                self.data = (self.data & !mask) | (value as u32) << 24;
            }
            1 => {
                let mask: u32 = 0xFF << 16;
                self.data = (self.data & !mask) | (value as u32) << 16;
            }
            2 => {
                let mask: u32 = 0xFF << 8;
                self.data = (self.data & !mask) | (value as u32) << 8;
            }
            3 => {
                let mask: u32 = 0xFF;
                self.data = (self.data & !mask) | (value as u32) << 0;
            }
            _ => {
                panic!("invalid register!");
            }
        }
    }
}
