// use crate::apu::registers::Registers;

use crate::apu::units::length_counter::LengthCounter;
use crate::apu::units::sequence_timer::SequenceTimer;

pub struct TriangleChannel {
    sequence_timer: SequenceTimer,
    length_counter: LengthCounter,

    linear_counter_reload_flag: bool,
    sequence_index: u8,

    // $4008
    linear_counter_control_flag: bool,    // C (1 bit)
    linear_counter_reload_value: u8, // RRRR RRR (7 bits)
    linear_counter_value: u8,

}

impl TriangleChannel {
    pub fn new() -> TriangleChannel {
        TriangleChannel {
            sequence_timer: SequenceTimer::new(),
            length_counter: LengthCounter::new(),

            linear_counter_reload_flag: false,
            sequence_index: 0,

            linear_counter_control_flag: false,
            linear_counter_reload_value: 0,
            linear_counter_value: 0,
        }
    }
    pub fn write_4008(&mut self, value: u8) {
        self.linear_counter_control_flag = (value & 0b1000_0000) != 0;
        self.linear_counter_reload_value = value & 0b0111_1111;

        self.length_counter.set_halt(true);
    }

    pub fn write_400a(&mut self, value: u8) {
        // LLLL LLLL (8 bits)
        self.sequence_timer.set_reload_low(value);
        // self.timer_low = value;
    }

    pub fn write_400b(&mut self, value: u8) {
        let length_counter_load = value >> 3; // upper 5 bits
        self.length_counter.set(length_counter_load);

        let timer_high = (value & 0b0000_0111) as u8; // lower 3 bits
        self.sequence_timer.set_reload_high(timer_high);

        self.linear_counter_reload_flag = true;
    }

    pub fn clock(&mut self, quarter_frame_clock: bool) {
        let advance_waveform = self.sequence_timer.clock();
        if advance_waveform && self.linear_counter_value > 0 && self.length_counter.output() > 0 {
            self.sequence_index = (self.sequence_index + 1) % 32;
        }

        if quarter_frame_clock {
            if self.linear_counter_reload_flag {
                self.linear_counter_value = self.linear_counter_reload_value;
            } else if self.linear_counter_value > 0 {
                self.linear_counter_value -= 1;
            }

            if self.linear_counter_control_flag == false {
                self.linear_counter_reload_flag = false;
            }
            println!("TriangleChannel: seq_ind {}\tlinear_ct: {}\tlength_ct:{}", self.sequence_index, self.linear_counter_value, self.length_counter.output());
        }
    }

    pub fn sample(&self) -> u8 {
        const TRIANGLE_TABLE: [u8; 32] = [15, 14, 13, 12, 11, 10,  9,  8,  7,  6,  5,  4,  3,  2,  1,  0,
        0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13, 14, 15];

        let value = TRIANGLE_TABLE[self.sequence_index as usize];
        value
    }
}
