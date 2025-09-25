use crate::apu::units::envelope::Envelope;
use crate::apu::units::length_counter::LengthCounter;
use crate::apu::units::sequence_timer::SequenceTimer;
use crate::apu::units::sweep::{PulseType, Sweep};
use sdl2::timer::Timer;

pub struct PulseChannel {
    seq_timer: SequenceTimer,
    length_counter: LengthCounter,
    envelope: Envelope,
    sweep: Sweep,

    duty_cycle: u8,
    sequence: u8,
}

impl PulseChannel {
    pub fn new(is_channel1: bool) -> PulseChannel {
        let pulse_type = if is_channel1 {
            PulseType::Pulse1
        } else {
            PulseType::Pulse2
        };
        PulseChannel {
            seq_timer: SequenceTimer::new(),
            sweep: Sweep::new(pulse_type),
            envelope: Envelope::new(),
            duty_cycle: 0,
            sequence: 0,
            length_counter: LengthCounter::new(),
        }
    }

    /*
        0x4000 : Pulse1 Main register
            7654 3210
            DDLC VVVV
                DD: Duty cycle.
                L : Loop. If set, its counter will not decrease,
                resulting in a tone that plays continuously.
                C: Const volume. If 1, the sweep will not change its
                volume over time.
                VVVV: Volume (C=1) or Envelope(C=0)
    */
    pub fn write_4000(&mut self, value: u8) {
        println!("write_4000({:08b}", value);
        self.duty_cycle = (value & 0b1100_0000) >> 6;
        match self.duty_cycle {
            0 => self.sequence = 0b0100_0000,
            1 => self.sequence = 0b0110_0000,
            2 => self.sequence = 0b0111_1000,
            3 => self.sequence = 0b1001_1111,
            _ => {
                panic!("invalid duty cycle!");
            }
        };

        self.envelope.set(value);
    }

    /*
       0x4001: Sweep controls
           7654 3210
           EPPP NSSS
               E: Enable
               P: Period
               N: Negate or flip
               S: Shift
    */
    pub fn write_4001(&mut self, value: u8) {
        self.sweep.set(value);
    }

    // 0x4002 : Timer lower bits
    pub fn write_4002(&mut self, value: u8) {
        self.seq_timer.set_reload_low(value);
    }

    /*
       0x4003 : Length & Timer upper bits
          LLLL LTTT
               L: Length
               T: Upper timer bits.
    */
    pub fn write_4003(&mut self, value: u8) {
        let length_counter_load = (value & 0b1111_1000) >> 3;
        self.length_counter.set(length_counter_load);
        self.seq_timer.set_reload_high(value & 0b111);

        // Restart envelope + sequence
        self.envelope.start();
        self.seq_timer.reset();
    }
}

impl PulseChannel {
    /// Clocked every APU cycle (1/2 CPU)
    pub fn clock(&mut self, quarter_frame_clock: bool, half_frame_clock: bool) {
        // Check if timer clocks waveform
        if self.seq_timer.clock() {
            // Advance duty cycle
            self.sequence = (self.sequence << 1) | (self.sequence >> 7);
            // println!("sequence: {:08b}", self.sequence);
        }

        // Clock envelope
        if quarter_frame_clock {
            self.envelope.clock();
        }

        // Clock length counter and sweep
        if half_frame_clock {
            self.length_counter.clock();
            let mut seq_timer_reload = self.seq_timer.output();
            self.sweep.clock(&mut seq_timer_reload);
            self.seq_timer.set_reload(seq_timer_reload);
        }
    }

    fn duty_output(&self) -> u8 {
        if (self.sequence & 0b1000_0000) != 0 {
            1
        } else {
            0
        }
    }

    pub fn sample(&self) -> u8 {
        let duty_output = self.duty_output();
        let vol = self.envelope.output();
        let length_counter = self.length_counter.output();

        if duty_output == 0 || length_counter == 0 {
            0
        } else {
            vol
        }
    }
}
