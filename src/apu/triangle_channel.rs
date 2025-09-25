// use crate::apu::registers::Registers;

pub struct TriangleChannel {
    // $4008
    pub linear_counter_control: bool,    // C (1 bit)
    pub linear_counter_reload_value: u8, // RRRR RRR (7 bits)

    // $400A
    pub timer_low: u8, // LLLL LLLL (8 bits)

    // $400B
    pub length_counter_load: u8, // LLLL L (5 bits)
    pub timer_high: u8,          // HHH (3 bits)
}

impl TriangleChannel {
    pub fn new() -> TriangleChannel {
        TriangleChannel {
            linear_counter_control: false,
            linear_counter_reload_value: 0,
            timer_low: 0,
            length_counter_load: 0,
            timer_high: 0,
        }
    }
    pub fn write_4008(&mut self, value: u8) {
        self.linear_counter_control = (value & 0b1000_0000) != 0;
        self.linear_counter_reload_value = value & 0b0111_1111;
    }

    pub fn write_400a(&mut self, value: u8) {
        self.timer_low = value;
    }

    pub fn write_400b(&mut self, value: u8) {
        self.length_counter_load = value >> 3; // upper 5 bits
        self.timer_high = (value & 0b0000_0111) as u8; // lower 3 bits
    }

    pub fn clock(&mut self) {}
}
