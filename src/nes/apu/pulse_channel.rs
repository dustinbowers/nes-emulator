use super::units::envelope::Envelope;
use super::units::length_counter::LengthCounter;
use super::units::sequence_timer::SequenceTimer;
use super::units::sweep::{PulseType, Sweep};

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
        // println!("write_4000({:08b}", value);
        self.duty_cycle = (value & 0b1100_0000) >> 6;
        self.sequence = match self.duty_cycle {
            0 => 0b0100_0000,
            1 => 0b0110_0000,
            2 => 0b0111_1000,
            3 => 0b1011_1111,
            _ => 0b0100_0000,
        };

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
        self.length_counter.set(length_counter_load);
        self.seq_timer.set_reload_high(value & 0b111);
        // println!(
        //     "4003 write: length={}, timer_high={}, reload_value={}",
        //     length_counter_load,
        //     value & 0x07,
        //     self.seq_timer.reload_value
        // );
        // Restart envelope + sequence
        self.envelope.set_start_flag(true);
        self.seq_timer.reset();
    }
}

impl PulseChannel {
    pub fn disable(&mut self) {
        self.length_counter.set_enabled(false);
        self.length_counter.set_halt(true);
    }

    pub fn is_enabled(&self) -> bool {
        self.length_counter.output() > 0
    }

    /// Clocked every APU cycle (1/2 CPU)
    pub fn clock(&mut self, quarter_frame_clock: bool, half_frame_clock: bool) {
        // Check if timer clocks waveform
        let advance_waveform = self.seq_timer.clock();
        if advance_waveform {
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
            let mut seq_timer_reload = self.seq_timer.get_reload();
            self.sweep.clock(&mut seq_timer_reload);
            self.seq_timer.set_reload(seq_timer_reload);
        }
    }

    pub fn sample(&self) -> u8 {
        let seq_active = (self.sequence & 0b1000_0000) != 0;
        let vol = self.envelope.output();
        let len = self.length_counter.output();
        if !seq_active || len == 0 || self.seq_timer.output() < 8 {
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

    // Helper function to create a new PulseChannel for testing
    fn setup_pulse_channel(is_channel1: bool) -> PulseChannel {
        PulseChannel::new(is_channel1)
    }

    #[test]
    fn test_initial_state() {
        let channel1 = setup_pulse_channel(true);
        assert_eq!(channel1.duty_cycle, 0);
        assert_eq!(channel1.sequence, 0);
        assert_eq!(channel1.seq_timer.output(), 0);
        assert_eq!(channel1.length_counter.output(), 0);
        assert_eq!(channel1.envelope.output(), 0); // Envelope should be off initially
        assert_eq!(channel1.sweep.is_enabled(), false);
        assert_eq!(channel1.sweep.get_period(), 0);
        assert_eq!(channel1.sweep.get_shift(), 0);
        assert_eq!(channel1.sample(), 0);

        let channel2 = setup_pulse_channel(false);
        assert_eq!(channel2.duty_cycle, 0);
    }

    #[test]
    fn test_write_4000_duty_cycle_and_sequence() {
        let mut channel = setup_pulse_channel(true);

        // Duty cycle 0 (0b0100_0000)
        channel.write_4000(0b0000_0000);
        assert_eq!(channel.duty_cycle, 0);
        assert_eq!(channel.sequence, 0b0100_0000);

        // Duty cycle 1 (0b0110_0000)
        channel.write_4000(0b0100_0000);
        assert_eq!(channel.duty_cycle, 1);
        assert_eq!(channel.sequence, 0b0110_0000);

        // Duty cycle 2 (0b0111_1000)
        channel.write_4000(0b1000_0000);
        assert_eq!(channel.duty_cycle, 2);
        assert_eq!(channel.sequence, 0b0111_1000);

        // Duty cycle 3 (0b1011_1111)
        channel.write_4000(0b1100_0000);
        assert_eq!(channel.duty_cycle, 3);
        assert_eq!(channel.sequence, 0b1011_1111);
    }

    #[test]
    fn test_write_4000_envelope_settings() {
        let mut channel = setup_pulse_channel(true);

        // Constant volume, volume 10
        channel.write_4000(0b0001_1010);
        assert_eq!(channel.envelope.get_volume_mode(), VolumeMode::Constant);
        assert_eq!(channel.envelope.get_divider_period(), 10);
        assert_eq!(channel.envelope.output(), 10); // Volume should be 10 for constant mode

        // Envelope mode, period 5, loop enabled
        channel.write_4000(0b0010_0101);
        assert_eq!(channel.envelope.get_volume_mode(), VolumeMode::Envelope);
        assert_eq!(channel.envelope.get_divider_period(), 5);
        assert_eq!(channel.envelope.get_loop_flag(), true);
    }

    #[test]
    fn test_write_4001_sweep_settings() {
        let mut channel = setup_pulse_channel(true);

        // Sweep enabled, period 3, negate, shift 2
        channel.write_4001(0b1011_1010);
        assert_eq!(channel.sweep.is_enabled(), true);
        assert_eq!(channel.sweep.get_period(), 3);
        assert_eq!(channel.sweep.get_negate_flag(), true);
        assert_eq!(channel.sweep.get_shift(), 2);

        // Sweep disabled, period 0, not negate, shift 0
        channel.write_4001(0b0000_0000);
        assert_eq!(channel.sweep.is_enabled(), false);
        assert_eq!(channel.sweep.get_period(), 0);
        assert_eq!(channel.sweep.get_negate_flag(), false);
        assert_eq!(channel.sweep.get_shift(), 0);
    }

    #[test]
    fn test_write_4002_timer_low() {
        let mut channel = setup_pulse_channel(true);
        channel.write_4002(0x12);
        // The reload value will be affected by 4003 as well, so we only test the low bits.
        // We'll have a more comprehensive test for the full timer reload value later.
        assert_eq!(channel.seq_timer.get_reload_low_bits(), 0x12);
    }

    #[test]
    fn test_write_4003_length_counter_and_timer_high() {
        let mut channel = setup_pulse_channel(true);

        // Write timer low bits first
        channel.write_4002(0x0F); // Timer low 0b00001111

        // Write 4003: Length=10 (0b01010), Timer high=5 (0b101)
        channel.write_4003(0b0101_0101); // 0x55

        // Check the length counter output
        assert_eq!(channel.length_counter.output(), 60); // Length counter should be set to 10

        // Check the timer high bits
        assert_eq!(channel.seq_timer.get_reload_high_bits(), 0b101);

        // Check the timer reload value
        assert_eq!(channel.seq_timer.output(), (0b101 << 8) | 0x0F); // reload_value = (high << 8) | low

        // When 0x4003 is written, the timer is reset,
        // so the current timer value becomes reload_value.
        assert_eq!(channel.seq_timer.output(), (0b101 << 8) | 0x0F); // Ensure timer is reset to reload value

        // Also, envelope is started
        assert_eq!(channel.envelope.get_start_flag(), true);
    }

    // #[test]
    // fn test_timer_clocking_waveform() {
    //     let mut channel = setup_pulse_channel(true);
    //     channel.write_4000(0b0000_0000); // duty 0: sequence starts as 0b01000000
    //     channel.write_4002(0x01);        // timer low
    //     channel.write_4003(0x00);        // timer high = 0, length counter = 0
    //
    //     // Timer reload value = (0 << 8) | 1 = 1
    //     // Effective period = (1 + 1) * 2 = 2 APU cycles per waveform step
    //
    //     // Initial state
    //     assert_eq!(channel.sequence, 0b0100_0000);
    //
    //
    // }
    //

    #[test]
    fn test_envelope_clocking() {
        let mut channel = setup_pulse_channel(true);
        // Envelope mode, period 2, initial volume 5, loop enabled
        channel.write_4000(0b0010_0010); // Duty 0, L=1, C=0, VVVV=2
        channel.write_4003(0x00); // Trigger envelope restart

        // Initial state after trigger
        assert_eq!(channel.envelope.output(), 2); // Initial output should be volume

        // Clock quarter frame, no change yet (divider not clocked enough)
        channel.clock(true, false);
        assert_eq!(channel.envelope.output(), 2);

        // Clock quarter frame again, envelope should decay if not looping
        // The envelope divider period is 2. So it should clock every 3rd clock.
        channel.clock(true, false); // Divider 1
        channel.clock(true, false); // Divider 2, clocks envelope
        assert_eq!(channel.envelope.output(), 1); // Volume should have decremented

        channel.clock(true, false); // Divider 1
        channel.clock(true, false); // Divider 2, clocks envelope
        assert_eq!(channel.envelope.output(), 0); // Volume should have decremented to 0

        // With loop enabled, it should reset to 2
        channel.clock(true, false); // Divider 1
        channel.clock(true, false); // Divider 2, clocks envelope
        assert_eq!(channel.envelope.output(), 2);
    }

    #[test]
    fn test_length_counter_clocking() {
        let mut channel = setup_pulse_channel(true);
        channel.write_4003(0b0000_1000); // Length L=1 (8) -> length counter loads 16
        channel.write_4000(0b0000_0000); // Enable sound

        assert_eq!(channel.length_counter.output(), 16);

        // Clock half frame (length counter clocks)
        channel.clock(false, true);
        assert_eq!(channel.length_counter.output(), 15);

        // Continue clocking until it reaches 0
        for _ in 0..14 {
            channel.clock(false, true);
        }
        assert_eq!(channel.length_counter.output(), 0);

        // Once at 0, it stays at 0
        channel.clock(false, true);
        assert_eq!(channel.length_counter.output(), 0);

        // Test with loop flag (L bit in 0x4000)
        channel.write_4000(0b0100_0000); // Duty 1, L=1
        channel.write_4003(0b0000_1000); // Reset length counter
        assert_eq!(channel.length_counter.output(), 16);

        for _ in 0..100 {
            channel.clock(false, true);
        }
        assert_eq!(channel.length_counter.output(), 16); // Should not decrement
    }

    #[test]
    fn test_sweep_unit_clocking() {
        let mut channel = setup_pulse_channel(true);
        // Sweep enabled, period 2, negate (true), shift 1
        channel.write_4001(0b1010_1001); // E=1, P=2, N=1, S=1
        channel.write_4002(0x40); // Timer low (0x40)
        channel.write_4003(0x00); // Timer high (0) => reload = 0x40 (64)

        let initial_timer_reload = channel.seq_timer.output(); // 65 (0x40 + 1)

        // Half frame clock 1 (sweep divider period 0 - 2, so it clocks on 3rd clock)
        channel.clock(false, true); // Divider 1
        assert_eq!(channel.seq_timer.output(), initial_timer_reload); // No change yet

        channel.clock(false, true); // Divider 2
        assert_eq!(channel.seq_timer.output(), initial_timer_reload); // No change yet

        // Half frame clock 3 (sweep clocks)
        channel.clock(false, true);
        let expected_shift = initial_timer_reload >> 1; // shift 1
        let expected_new_timer = initial_timer_reload - expected_shift - 1; // Pulse 1 negate formula
        assert_eq!(channel.seq_timer.output(), expected_new_timer);

        // Test with Pulse2 (different negate behavior)
        let mut channel2 = setup_pulse_channel(false);
        // Sweep enabled, period 2, negate (true), shift 1
        channel2.write_4001(0b1010_1001); // E=1, P=2, N=1, S=1
        channel2.write_4002(0x40); // Timer low (0x40)
        channel2.write_4003(0x00); // Timer high (0) => reload = 0x40 (64)
        let initial_timer_reload_ch2 = channel2.seq_timer.output(); // 65

        channel2.clock(false, true); // Divider 1
        channel2.clock(false, true); // Divider 2
        channel2.clock(false, true); // Sweep clocks
        let expected_shift_ch2 = initial_timer_reload_ch2 >> 1; // shift 1
        let expected_new_timer_ch2 = initial_timer_reload_ch2 - expected_shift_ch2; // Pulse 2 negate formula
        assert_eq!(channel2.seq_timer.output(), expected_new_timer_ch2);
    }

    #[test]
    fn test_sample_output_conditions() {
        let mut channel = setup_pulse_channel(true);

        // Case 1: Sequence not active (current bit is 0)
        channel.write_4000(0b0000_0000); // Duty 0, sequence 0b01000000
        channel.write_4002(0x01); // Timer low
        channel.write_4003(0x00); // Timer high 0, length counter 0
        channel.envelope.set_volume(10); // Set envelope to 10 for testing
        assert_eq!(channel.sample(), 10); // First bit is 0, but current sample is 0b01000000 -> 0 at bit 7 -> 0

        // Clock sequence to make MSB 1
        channel.clock(false, false); // Timer clocks at 2. So one clock will shift.
        channel.clock(false, false); // This is the clock that will actually shift it
        assert_eq!(channel.sequence, 0b1000_0000);
        assert_eq!(channel.sample(), 10);

        // Case 2: Length counter is 0
        channel.write_4000(0b0000_0000); // Reset for next test
        channel.write_4002(0x00);
        channel.write_4003(0x00); // Length counter loads 0
        channel.envelope.set_volume(10); // Set volume
        channel.sequence = 0b1000_0000; // Ensure sequence is active
        assert_eq!(channel.length_counter.output(), 0);
        assert_eq!(channel.sample(), 0); // Should be muted

        // Case 3: Timer output is less than 8 (i.e., too high frequency)
        // A timer reload value of N results in a period of N+1.
        // A period of < 8 (reload value < 7) results in silence.
        channel.write_4000(0b0000_0000);
        channel.write_4003(0b0000_1000); // Length counter != 0
        channel.envelope.set_volume(10);
        channel.sequence = 0b1000_0000;

        // Set timer to reload_value = 0 (period 1)
        channel.write_4002(0x00);
        assert_eq!(channel.seq_timer.output(), 1); // Current timer value is reload + 1
        assert!(channel.seq_timer.output() < 8);
        assert_eq!(channel.sample(), 0); // Muted

        // Set timer to reload_value = 7 (period 8)
        channel.write_4002(0x07);
        assert_eq!(channel.seq_timer.output(), 8);
        assert_eq!(channel.sample(), 10); // Not muted

        // Case 4: All conditions met
        channel.write_4000(0b0000_0000); // Duty 0, L=0, C=0, VVVV=0 (envelope active)
        channel.envelope.set_volume(5); // Set envelope to 5
        channel.write_4003(0b0000_1000); // Length counter load = 16
        channel.write_4002(0x10); // Timer reload 16 (period 17)
        channel.sequence = 0b1000_0000; // Sequence active
        assert_eq!(channel.sample(), 5);
    }

    #[test]
    fn test_sweep_muting() {
        let mut channel = setup_pulse_channel(true);
        channel.write_4000(0b0000_0000); // Enable sound
        channel.write_4003(0b0000_1000); // Length counter active
        channel.envelope.set_volume(10); // Output volume 10
        channel.sequence = 0b1000_0000; // Sequence active

        // Initial timer reload = 0x0100 (256) (period 257)
        channel.write_4002(0x00); // Low
        channel.write_4003(0x01); // High (sets 0x0100) -> 0x0100 + 1 = 257

        // Sweep enabled, period 0, negate, shift 7
        channel.write_4001(0b1000_1111); // E=1, P=0, N=1, S=7
                                         // The period of 0 means the sweep unit will clock on the next
                                         // half-frame.
        assert_eq!(channel.sweep.is_enabled(), true);
        assert_eq!(channel.sweep.get_period(), 0);
        assert_eq!(channel.sweep.get_negate_flag(), true);
        assert_eq!(channel.sweep.get_shift(), 7);

        // Before sweep clocks, it should produce sound
        assert_eq!(channel.sample(), 10);

        // Clock half-frame, sweep should apply
        channel.clock(false, true);

        // Calculate expected new timer value after sweep
        // Pulse 1: timer = timer_period - (timer_period >> shift) - 1
        // initial_timer_reload = 0x0100 (256). The `output` of `SequenceTimer` gives `reload_value + 1`.
        // So `timer_period` from sweep's perspective should be `channel.seq_timer.output()`
        let current_timer_period = 257; // 0x0100 + 1
        let shifted_value = current_timer_period >> 7; // 257 >> 7 = 2
        let new_timer_period_minus_1 = current_timer_period as i16 - shifted_value as i16 - 1; // 257 - 2 - 1 = 254
        let expected_new_reload_value = new_timer_period_minus_1 - 1; // 254 - 1 = 253 (0xFD)
        let new_timer_value_from_sweep = channel.seq_timer.output();

        // After one sweep update, the timer period has decreased.
        // It should still be within bounds (>= 8 and <= 0x7FF)
        assert!(new_timer_value_from_sweep >= 8);
        assert!(new_timer_value_from_sweep <= 0x7FF);
        assert_eq!(channel.sample(), 10); // Still active

        // Now set shift to a high value that would mute (e.g., period too low)
        channel.write_4002(0x00);
        channel.write_4003(0x00); // Timer reload 0 (period 1)
        channel.sweep.set_enabled(true);
        channel.sweep.set_shift(0); // Shift 0, period won't change too much initially
        channel.sweep.set_period(0); // Clock immediately
        channel.clock(false, true); // Clock sweep (timer output is now 1)

        // Check for muting condition: timer_reload_value < 8 or timer_reload_value > 0x7FF
        // If the initial period is 1, and shift is 0, the new period will be 1. This should mute.
        assert_eq!(channel.seq_timer.output(), 1);
        assert_eq!(channel.sample(), 0); // Should be muted due to too low frequency
    }
}
