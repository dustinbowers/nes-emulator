use super::units::envelope::Envelope;
use super::units::length_counter::LengthCounter;
use super::units::sequence_timer::SequenceTimer;
use super::units::sweep::{PulseType, Sweep};
use crate::nes::apu::FrameClock;

// See: https://www.nesdev.org/wiki/APU_Pulse#Pulse_channel_output_to_mixer
const DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 1, 0, 0, 0, 0, 0, 0],
    [0, 1, 1, 0, 0, 0, 0, 0],
    [0, 1, 1, 1, 1, 0, 0, 0],
    [1, 0, 0, 1, 1, 1, 1, 1],
];

pub struct PulseChannel {
    seq_timer: SequenceTimer,
    pub length_counter: LengthCounter,
    envelope: Envelope,
    sweep: Sweep,

    duty_cycle: u8,
    duty_step: u8,
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
            duty_step: 0,
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
        self.duty_cycle = (value & 0b1100_0000) >> 6;

        let length_counter_halt = value & 0b0010_0000 != 0;
        self.length_counter.set_halt(length_counter_halt);

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
        self.length_counter.load_index(length_counter_load);

        self.seq_timer.set_reload_high(value & 0b111);

        // restart envelope and sequencer
        self.envelope.set_start_flag(true);
        self.seq_timer.reset();

        // reset duty sequencer phase
        self.duty_step = 0;
    }
}

impl PulseChannel {
    pub fn set_enabled(&mut self, enabled: bool) {
        self.length_counter.set_enabled(enabled);
    }

    pub fn length_active(&self) -> bool {
        self.length_counter.output() > 0
    }

    /// Clocked every APU cycle (1/2 CPU)
    pub fn clock(&mut self, frame_clock: &FrameClock, timer_tick: bool) {
        // Check if timer clocks waveform
        if timer_tick && self.seq_timer.clock() {
            // Advance duty cycle
            self.duty_step = (self.duty_step + 1) & 7;
        }

        // Clock envelope
        if frame_clock.is_quarter() {
            self.envelope.clock();
        }

        // Clock length counter and sweep
        if frame_clock.is_half() {
            self.length_counter.clock();
            let mut seq_timer_reload = self.seq_timer.get_reload();
            self.sweep.clock(&mut seq_timer_reload);
            self.seq_timer.set_reload(seq_timer_reload);
        }
    }

    pub fn sample(&self) -> u8 {
        // let seq_active = (self.duty_step & 0b1000_0000) != 0;
        let duty_cycle = self.duty_cycle as usize;
        let duty_step = self.duty_step as usize;
        let seq_active = DUTY_TABLE[duty_cycle][duty_step] != 0;

        let vol = self.envelope.output();
        let len = self.length_counter.output();
        let reload = self.seq_timer.get_reload();

        // pulse is silenced if the timer period (11-bit reload) is < 8
        if !seq_active || len == 0 || reload < 8 || self.sweep.is_muting(reload) {
            0
        } else {
            vol
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::units::envelope::VolumeMode;
    use super::*;

    fn make_constant_pulse(ch1: bool, duty: u8) -> PulseChannel {
        let mut p = PulseChannel::new(ch1);
        p.set_enabled(true);

        // DDLC_VVVV : constant volume mode (C=1), volume=15, length halt doesn't matter here
        // Set duty bits
        let v = (duty << 6) | 0b0001_1111;
        p.write_4000(v);

        // Timer = 8 (>=8 so not auto-silenced), and load a length
        p.write_4002(8);
        p.write_4003(0b0001_1000); // length index=3, timer_hi=0
        p
    }

    fn advance_one_step(p: &mut PulseChannel) {
        let reload = p.seq_timer.get_reload();
        for _ in 0..(reload + 1) {
            p.clock(&FrameClock::None, true);
        }
    }

    /// Return 1 if channel volume > 0, otherwise 0
    fn bit(p: &PulseChannel) -> u8 {
        (p.sample() > 0) as u8
    }

    #[test]
    fn test_initial_state() {
        let mut ch1 = PulseChannel::new(true);
        assert_eq!(ch1.duty_cycle, 0);
        assert_eq!(ch1.duty_step, 0);
        assert_eq!(ch1.seq_timer.output(), 0);
        assert_eq!(ch1.length_counter.output(), 0);
        assert_eq!(ch1.envelope.output(), 0); // Envelope should be off initially
        assert_eq!(ch1.sample(), 0);

        let mut ch2 = PulseChannel::new(false);
        assert_eq!(ch2.duty_cycle, 0);
        assert_eq!(ch2.duty_step, 0);
        assert_eq!(ch2.seq_timer.output(), 0);
        assert_eq!(ch2.length_counter.output(), 0);
        assert_eq!(ch2.envelope.output(), 0); // Envelope should be off initially
        assert_eq!(ch2.sample(), 0);
    }

    #[test]
    fn test_write_4000_envelope_settings() {
        let mut ch = make_constant_pulse(true, 2);

        // Constant volume, volume 10
        ch.write_4000(0b0001_1010);
        assert_eq!(ch.envelope.get_volume_mode(), VolumeMode::Constant);
        assert_eq!(ch.envelope.get_divider_period(), 10);
        assert_eq!(ch.envelope.output(), 10); // Volume should be 10 for constant mode

        // Envelope mode, period 5, loop enabled
        ch.write_4000(0b0010_0101);
        assert_eq!(ch.envelope.get_volume_mode(), VolumeMode::Envelope);
        assert_eq!(ch.envelope.get_divider_period(), 5);
        assert_eq!(ch.envelope.get_loop_flag(), true);
    }

    #[test]
    fn write_4000_sets_duty_and_resets_phase() {
        let mut ch = PulseChannel::new(true);
        ch.set_enabled(true);

        ch.write_4000(0b1000_0000 | 0b0001_1111); // duty=2 constant vol
        assert_eq!(ch.duty_cycle, 2);
        assert_eq!(ch.duty_step, 0);
    }

    #[test]
    fn duty_step_advances_mod_8_on_timer_events() {
        let mut ch = PulseChannel::new(true);
        ch.set_enabled(true);

        ch.write_4000(0b0100_0000 | 0b0001_1111); // duty=1, constant vol
        ch.write_4002(8);
        ch.write_4003(0b0001_1000);

        let start = ch.duty_step;

        // force exactly one sequencer event
        let reload = ch.seq_timer.get_reload();
        for _ in 0..(reload + 1) {
            ch.clock(&FrameClock::None, true);
        }

        assert_eq!(ch.duty_step, (start + 1) & 7);
    }

    #[test]
    fn sample_matches_duty_table_over_8_steps() {
        let mut ch = PulseChannel::new(true);
        ch.set_enabled(true);

        // constant volume=15 so sample>0 means duty output=1
        ch.write_4000((2 << 6) | 0b0001_1111); // duty=2
        ch.write_4002(8);
        ch.write_4003(0b0001_1000);

        let mut bits = Vec::new();
        for _ in 0..8 {
            bits.push((ch.sample() > 0) as u8);

            // advance exactly one duty step
            let reload = ch.seq_timer.get_reload();
            for _ in 0..(reload + 1) {
                ch.clock(&FrameClock::None, true);
            }
        }

        assert_eq!(bits, DUTY_TABLE[2]);
    }

    #[test]
    fn test_write_4002_timer_low() {
        let mut ch = make_constant_pulse(true, 2);
        ch.write_4002(0x12);
        assert_eq!(ch.seq_timer.get_reload_low_bits(), 0x12);
    }

    #[test]
    fn write_4003_length_counter_and_timer_high() {
        let mut ch = make_constant_pulse(true, 2);
        ch.set_enabled(true);

        // Write timer low bits first
        ch.write_4002(0x0F); // Timer low 0b00001111

        // Write 4003: Length=10 (0b01010), Timer high=5 (0b101)
        ch.write_4003(0b0101_0101); // 0x55

        assert_eq!(ch.length_counter.output(), 60); // Length counter should be set to 10
        assert_eq!(ch.seq_timer.get_reload_high_bits(), 0b101);
        assert_eq!(ch.seq_timer.output(), (0b101 << 8) | 0x0F); // reload_value = (high << 8) | low

        // When 0x4003 is written, the timer is reset,
        // so the current timer value becomes reload_value.
        assert_eq!(ch.seq_timer.output(), (0b101 << 8) | 0x0F); // Ensure timer is reset to reload value
        assert_eq!(ch.envelope.get_start_flag(), true);
    }

    #[test]
    fn write_4003_resets_phase_to_zero() {
        let mut ch = PulseChannel::new(true);
        ch.set_enabled(true);

        ch.write_4000((3 << 6) | 0b0001_1111);
        ch.write_4002(8);
        ch.write_4003(0b0001_1000);

        // advance a couple steps
        for _ in 0..2 {
            let reload = ch.seq_timer.get_reload();
            for _ in 0..(reload + 1) {
                ch.clock(&FrameClock::None, true);
            }
        }
        assert_ne!(ch.duty_step, 0);

        // retrigger
        ch.write_4003(0b0001_1000);
        assert_eq!(ch.duty_step, 0);
    }

    #[test]
    fn sequencer_advances_every_reload_plus_one_timer_clocks() {
        let mut ch = make_constant_pulse(true, 2);
        ch.set_enabled(true);
        ch.write_4000(0b0001_1111); // constant volume
        ch.write_4002(0x00);
        ch.write_4003(0x01); // reload = 256
        ch.duty_step = 0b1000_0000;

        // Shouldn't advance for the next 256 clocks
        for _ in 0..256 {
            ch.clock(&FrameClock::None, true);
            assert_eq!(ch.duty_step, 0b1000_0000);
        }

        // Next clock advances once
        ch.clock(&FrameClock::None, true);
        assert_ne!(ch.duty_step, 0b1000_0000);
    }

    #[test]
    fn write_4003_resets_duty_phase() {
        let mut ch = make_constant_pulse(true, 2); // duty2 = 0b0000_1111

        // move into 1s of duty phase
        advance_one_step(&mut ch); // 0b0001_1110
        advance_one_step(&mut ch); // 0b0011_1100
        advance_one_step(&mut ch); // 0b0111_1000
        advance_one_step(&mut ch); // 0b1111_0000
        assert_eq!(bit(&ch), 1, "should be in high part of duty cycle");

        // retrigger the note
        ch.write_4003(0b0001_1000);

        // retrigger should reset duty phase to step 0
        assert_eq!(bit(&ch), 0, "should be in low part of duty cycle");
    }

    #[test]
    fn reload_below_8_silences() {
        let mut ch = make_constant_pulse(true, 3);
        ch.write_4002(7); // timer reload < 8
        ch.write_4003(0b0001_1000);

        // channel should be forced silent
        assert_eq!(ch.sample(), 0);
    }

    #[test]
    fn sweep_overflow_mutes_output() {
        let mut ch = make_constant_pulse(true, 3);

        // set timer to max
        ch.seq_timer.set_reload(0x7FF);
        ch.seq_timer.reset();

        // enable sweep, shift=1, negate=0
        ch.write_4001(0b1000_0001);

        // channel should be silenced
        assert_eq!(ch.sample(), 0);
    }
}
