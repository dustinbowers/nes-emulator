use super::units::length_counter::LengthCounter;
use super::units::sequence_timer::SequenceTimer;
use crate::nes::apu::FrameClock;

pub struct TriangleChannel {
    sequence_timer: SequenceTimer,
    length_counter: LengthCounter,

    linear_counter_reload_flag: bool,
    sequence_index: u8,

    // $4008
    linear_counter_control_flag: bool, // C (1 bit)
    linear_counter_reload_value: u8,   // RRRR RRR (7 bits)
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

        // when control flag is set, length counter halt is set; otherwise cleared.
        // bit 7 doubles as the length counter halt for triangle
        self.length_counter
            .set_halt(self.linear_counter_control_flag);
    }

    pub fn write_400a(&mut self, value: u8) {
        // LLLL LLLL (8 bits)
        self.sequence_timer.set_reload_low(value);
        // self.timer_low = value;
    }

    pub fn write_400b(&mut self, value: u8) {
        let length_counter_load = value >> 3; // upper 5 bits
        self.length_counter.load_index(length_counter_load);

        let timer_high = (value & 0b0000_0111) as u8; // lower 3 bits
        self.sequence_timer.set_reload_high(timer_high);

        self.linear_counter_reload_flag = true;
    }
}

impl TriangleChannel {
    pub fn set_enabled(&mut self, enabled: bool) {
        self.length_counter.set_enabled(enabled);
    }

    pub fn disable(&mut self) {
        self.length_counter.set_enabled(false);
    }

    pub fn length_active(&self) -> bool {
        self.length_counter.output() > 0
    }

    pub fn clock(&mut self, frame_clock: &FrameClock, timer_tick: bool) {
        // triangle advances only if both length counter and linear counter are non-zero,
        // and the timer period is at least 2
        let advance_waveform = timer_tick && self.sequence_timer.clock();
        if advance_waveform
            && self.linear_counter_value > 0
            && self.length_counter.output() > 0
            && self.sequence_timer.get_reload() >= 2
        {
            self.sequence_index = (self.sequence_index + 1) % 32;
        }

        if frame_clock.is_quarter() {
            if self.linear_counter_reload_flag {
                self.linear_counter_value = self.linear_counter_reload_value;
            } else if self.linear_counter_value > 0 {
                self.linear_counter_value -= 1;
            }

            // If control flag is clear, the reload flag is cleared on quarter frame clock
            if !self.linear_counter_control_flag {
                self.linear_counter_reload_flag = false;
            }
        }

        if frame_clock.is_half() {
            self.length_counter.clock();
        }
    }

    pub fn sample(&self) -> u8 {
        const TRIANGLE_TABLE: [u8; 32] = [
            15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10,
            11, 12, 13, 14, 15,
        ];

        if self.linear_counter_value == 0 || self.length_counter.output() == 0 {
            0
        } else {
            TRIANGLE_TABLE[self.sequence_index as usize]
        }
    }
}
