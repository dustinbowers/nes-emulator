use crate::nes::CPU_HZ_NTSC;
use crate::nes::apu::filter::OnePole;
use crate::nes::apu::output::ApuOutput;
use crate::nes::apu::status_register::ApuStatusRegister;
use dmc_channel::DmcChannel;
use noise_channel::NoiseChannel;
use pulse_channel::PulseChannel;
use std::cmp::PartialEq;
use thiserror::Error;
use triangle_channel::TriangleChannel;

mod dmc_channel;
mod filter;
mod noise_channel;
mod output;
mod pulse_channel;
mod status_register;
mod triangle_channel;
mod units;

const BLIP_GAIN: f32 = 5.0; // tune somewhere approx. 4..16
const DAC_SCALE: f32 = 32768.0; // i16 range

pub trait ApuBusInterface {
    fn apu_bus_read(&mut self, addr: u16) -> u8;
}

#[derive(Debug, Error)]
pub enum ApuError {
    #[error("Invalid APU register read: 0x{0:02X}")]
    InvalidRegisterRead(u16),

    #[error("Invalid APU register write: 0x{0:02X}")]
    InvalidRegisterWrite(u16),
}

/*
   mode 0:    mode 1:       function
   ---------  -----------  -----------------------------
    - - - f    - - - - -    IRQ (if bit 6 is clear)
    - l - l    - l - - l    Length counter and sweep
    e e e e    e e e - e    Envelope and linear counter
*/

#[derive(PartialEq)]
pub enum SequenceMode {
    Mode0,
    Mode1,
}

pub enum FrameClock {
    None,
    Quarter,
    QuarterAndHalf,
}

#[derive(Copy, Clone)]
pub enum ApuPhase {
    Even,
    Odd,
}

impl ApuPhase {
    pub fn new() -> Self {
        ApuPhase::Even
    }
    pub fn toggle(&mut self) {
        *self = match self {
            ApuPhase::Even => ApuPhase::Odd,
            ApuPhase::Odd => ApuPhase::Even,
        }
    }

    pub fn is_even(&self) -> bool {
        matches!(self, ApuPhase::Even)
    }

    pub fn is_odd(&self) -> bool {
        matches!(self, ApuPhase::Odd)
    }

    pub fn opposite(&self) -> Self {
        match self {
            ApuPhase::Even => ApuPhase::Odd,
            ApuPhase::Odd => ApuPhase::Even,
        }
    }
}

impl FrameClock {
    pub fn is_quarter(&self) -> bool {
        matches!(self, FrameClock::Quarter | FrameClock::QuarterAndHalf)
    }
    pub fn is_half(&self) -> bool {
        matches!(self, FrameClock::QuarterAndHalf)
    }

    pub fn reset() -> Self {
        FrameClock::QuarterAndHalf
    }
}

pub struct APU {
    bus: Option<*mut dyn ApuBusInterface>,

    cpu_phase: ApuPhase,
    seq_phase: ApuPhase,

    output: ApuOutput,
    last_dac: i32,

    // current_sample_raw: f32,
    pub pulse1: PulseChannel,
    pub pulse2: PulseChannel,
    pub triangle: TriangleChannel,
    pub noise: NoiseChannel,
    pub dmc: DmcChannel,

    pub status_register: ApuStatusRegister,

    pub mute_pulse1: bool,
    pub mute_pulse2: bool,
    pub mute_triangle: bool,
    pub mute_noise: bool,
    pub mute_dmc: bool,

    pub master_sequence_mode: SequenceMode,
    pub frame_clock_counter: u8,
    pub clock_counter: u32,
    pending_quarter_clock: bool,
    pending_half_clock: bool,
    pending_clock_reset: bool,
    pending_frame_reset_delay: u8,

    frame_irq_disable: bool,
    frame_irq_rising: bool,
    frame_irq_reassert: u8,

    sample_rate: f64,
    high_pass_90: OnePole,
    high_pass_440: OnePole,
    low_pass_14k: OnePole,

    pub error: Option<ApuError>,
}

impl Default for APU {
    fn default() -> Self {
        Self::new()
    }
}

impl APU {
    pub fn new() -> APU {
        APU {
            bus: None,
            cpu_phase: ApuPhase::new(),
            seq_phase: ApuPhase::new(),

            output: ApuOutput::new(CPU_HZ_NTSC, 44_100, 4096),
            last_dac: 0,
            // current_sample_raw: 0.0,

            // half_clock: false,
            pulse1: PulseChannel::new(true),
            pulse2: PulseChannel::new(false),
            triangle: TriangleChannel::new(),
            noise: NoiseChannel::new(),
            dmc: DmcChannel::new(),

            status_register: ApuStatusRegister::new(),

            mute_pulse1: false,
            mute_pulse2: false,
            mute_triangle: false,
            mute_noise: false,
            mute_dmc: false,

            master_sequence_mode: SequenceMode::Mode0,
            frame_clock_counter: 0,
            clock_counter: 0,
            pending_quarter_clock: false,
            pending_half_clock: false,
            pending_clock_reset: false,
            pending_frame_reset_delay: 0,

            frame_irq_disable: false,
            frame_irq_rising: false,
            frame_irq_reassert: 0,

            sample_rate: 44100.0, // A safe default
            high_pass_90: OnePole::default(),
            high_pass_440: OnePole::default(),
            low_pass_14k: OnePole::default(),

            error: None,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate;
        self.output.set_sample_rate(self.sample_rate as u32);
    }

    pub fn reset(&mut self) {
        self.cpu_phase = ApuPhase::new();
        self.seq_phase = ApuPhase::new();

        self.output.reset();
        self.last_dac = 0;

        self.pulse1 = PulseChannel::new(true);
        self.pulse2 = PulseChannel::new(false);
        self.triangle = TriangleChannel::new();
        self.noise = NoiseChannel::new();
        self.dmc = DmcChannel::new();

        self.status_register.update(0);

        self.mute_pulse1 = false;
        self.mute_pulse2 = false;
        self.mute_triangle = false;
        self.mute_noise = false;
        self.mute_dmc = false;

        self.master_sequence_mode = SequenceMode::Mode0;
        self.frame_clock_counter = 0;
        self.clock_counter = 0;
        self.pending_quarter_clock = false;
        self.pending_half_clock = false;
        self.pending_clock_reset = false;
        self.pending_frame_reset_delay = 0;

        self.frame_irq_disable = false;
        self.frame_irq_rising = false;
        self.frame_irq_reassert = 0;

        // self.sample_rate = 0.0; // Preserve through resets
        self.high_pass_90 = OnePole::default();
        self.high_pass_440 = OnePole::default();
        self.low_pass_14k = OnePole::default();

        self.error = None;
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        println!("APU::read({:04X})", addr);
        match addr {
            0x4015 => {
                // Status Register
                // Collect channel statuses
                self.status_register.set(
                    ApuStatusRegister::PULSE_CHANNEL_1,
                    self.pulse1.length_active(),
                );
                self.status_register.set(
                    ApuStatusRegister::PULSE_CHANNEL_2,
                    self.pulse2.length_active(),
                );
                self.status_register.set(
                    ApuStatusRegister::TRIANGLE_CHANNEL,
                    self.triangle.length_active(),
                );
                self.status_register
                    .set(ApuStatusRegister::NOISE_CHANNEL, self.noise.length_active());
                self.status_register
                    .set(ApuStatusRegister::DMC_CHANNEL, self.dmc.is_enabled());
                let output = self.status_register.bits();

                // Reading status register clears the frame interrupt flag (but not the DMC interrupt flag).
                // Quirk: If an interrupt flag was set at the same moment of the read, it will read back as 1 but it will not be cleared.
                if !self.frame_irq_rising {
                    self.status_register
                        .remove(ApuStatusRegister::FRAME_INTERRUPT);
                }

                output
            }
            _ => {
                self.error = Some(ApuError::InvalidRegisterRead(addr));
                0
            }
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x4000 => self.pulse1.write_4000(value),
            0x4001 => self.pulse1.write_4001(value),
            0x4002 => self.pulse1.write_4002(value),
            0x4003 => self.pulse1.write_4003(value),

            0x4004 => self.pulse2.write_4000(value),
            0x4005 => self.pulse2.write_4001(value),
            0x4006 => self.pulse2.write_4002(value),
            0x4007 => self.pulse2.write_4003(value),

            0x4008 => self.triangle.write_4008(value),
            0x4009 => { /* unused */ }
            0x400A => self.triangle.write_400a(value),
            0x400B => self.triangle.write_400b(value),

            0x400C => self.noise.write_400c(value),
            0x400D => { /* unused */ }
            0x400E => self.noise.write_400e(value),
            0x400F => self.noise.write_400f(value),

            0x4010 => self.dmc.write_4010(value),
            0x4011 => self.dmc.write_4011(value),
            0x4012 => self.dmc.write_4012(value),
            0x4013 => self.dmc.write_4013(value),

            0x4015 => {
                // Status Register
                let new_status = ApuStatusRegister::from_bits_truncate(value);
                let enable_pulse1 = new_status.contains(ApuStatusRegister::PULSE_CHANNEL_1);
                let enable_pulse2 = new_status.contains(ApuStatusRegister::PULSE_CHANNEL_2);
                let enable_triangle = new_status.contains(ApuStatusRegister::TRIANGLE_CHANNEL);
                let enable_noise = new_status.contains(ApuStatusRegister::NOISE_CHANNEL);
                let enable_dmc = new_status.contains(ApuStatusRegister::DMC_CHANNEL);

                self.status_register
                    .set(ApuStatusRegister::PULSE_CHANNEL_1, enable_pulse1);
                self.status_register
                    .set(ApuStatusRegister::PULSE_CHANNEL_2, enable_pulse2);
                self.status_register
                    .set(ApuStatusRegister::TRIANGLE_CHANNEL, enable_triangle);
                self.status_register
                    .set(ApuStatusRegister::NOISE_CHANNEL, enable_noise);
                self.status_register
                    .set(ApuStatusRegister::DMC_CHANNEL, enable_dmc);

                self.pulse1.set_enabled(enable_pulse1);
                self.pulse2.set_enabled(enable_pulse2);
                self.triangle.set_enabled(enable_triangle);
                self.noise.set_enabled(enable_noise);
                self.dmc.set_enabled(enable_dmc);

                // Writing to this register clears the DMC interrupt flag
                self.status_register
                    .remove(ApuStatusRegister::DMC_INTERRUPT);
            }
            0x4017 => {
                // Frame Counter
                /*
                   0x4017: MI-- ----
                       M: Mode.- bit 7
                       I: IRQ Off - bit 6
                */
                self.master_sequence_mode = match value & 0b1000_0000 != 0 {
                    false => SequenceMode::Mode0,
                    true => SequenceMode::Mode1,
                };
                self.frame_irq_disable = value & 0b0100_0000 != 0;

                // Writing to $4017 resets the frame counter and the quarter/half frame triggers happen
                // simultaneously, but only on "odd" cycles (and only after the first "even" cycle after the
                // write occurs) – thus, it happens either 2 or 3 cycles after the write (i.e. on the 2nd or
                // 3rd cycle of the next instruction). After 2 or 3 clock cycles (depending on when the write
                // is performed), the timer is reset.
                self.pending_frame_reset_delay = if self.cpu_phase.is_odd() { 3 } else { 4 };

                // If IRQs are disabled, clear interrupt flag and stop any IRQ asserts
                if self.frame_irq_disable {
                    self.status_register
                        .remove(ApuStatusRegister::FRAME_INTERRUPT);
                    self.frame_irq_reassert = 0;
                }
            }
            _ => {
                self.error = Some(ApuError::InvalidRegisterWrite(addr));
            }
        }
    }

    /// Clocks every CPU cycle
    pub fn clock(&mut self) {
        self.frame_irq_rising = false;

        // Frame clock triggers are delayed by 1 CPU cycle
        let mut frame_clock = FrameClock::None;
        if self.pending_quarter_clock || self.pending_half_clock {
            frame_clock = match (self.pending_quarter_clock, self.pending_half_clock) {
                (true, true) => FrameClock::QuarterAndHalf,
                (true, false) => FrameClock::Quarter,
                (false, true) => FrameClock::QuarterAndHalf,
                _ => unreachable!(),
            };
            self.pending_quarter_clock = false;
            self.pending_half_clock = false;
        }

        // Note: `just_reset` stops the sequencer from immediately ticking forward if it's reset
        //       in this cycle. We still need to strobe the channels though
        let mut just_reset = false;
        if self.pending_frame_reset_delay != 0 {
            self.pending_frame_reset_delay -= 1;
            if self.pending_frame_reset_delay == 0 {
                self.clock_counter = 0;
                just_reset = true;

                // Re-phase sequencer according to CPU phase
                self.seq_phase = self.cpu_phase.opposite();

                // If entering 5-step mode, reset event clocks quarter and half at the beginning
                if self.master_sequence_mode == SequenceMode::Mode1 {
                    self.pending_quarter_clock = true;
                    self.pending_half_clock = true;
                }
            }
        }

        let seq_tick = self.seq_phase.is_even();
        if !just_reset && seq_tick {
            if self.pending_clock_reset {
                self.pending_clock_reset = false;
                self.clock_counter = 0;
            } else {
                self.clock_counter += 1;
                match self.master_sequence_mode {
                    SequenceMode::Mode0 => {
                        // 4-step
                        match self.clock_counter {
                            3728 => {
                                self.pending_quarter_clock = true;
                            }
                            7456 => {
                                self.pending_quarter_clock = true;
                                self.pending_half_clock = true;
                            }
                            11185 => {
                                self.pending_quarter_clock = true;
                            }
                            14914 => {
                                self.pending_quarter_clock = true;
                                self.pending_half_clock = true;

                                // sequence ends
                                self.pending_clock_reset = true;

                                // IRQ on step boundary
                                if !self.frame_irq_disable {
                                    self.status_register
                                        .insert(ApuStatusRegister::FRAME_INTERRUPT);
                                    self.frame_irq_rising = true;

                                    // Quirk: When IRQ is triggered, it asserts for 3 consecutive cycles
                                    self.frame_irq_reassert = 2;
                                }
                            }
                            _ => {}
                        };
                    }
                    SequenceMode::Mode1 => {
                        // 5-step
                        match self.clock_counter {
                            3728 => {
                                self.pending_quarter_clock = true;
                            }
                            7456 => {
                                self.pending_quarter_clock = true;
                                self.pending_half_clock = true;
                            }
                            11185 => {
                                self.pending_quarter_clock = true;
                            }
                            14914 => { /* no clocks */ }
                            18640 => {
                                self.pending_quarter_clock = true;
                                self.pending_half_clock = true;

                                // sequence ends
                                self.pending_clock_reset = true;
                            }
                            _ => {}
                        };
                    }
                }
            }
        }

        let apu_tick = self.cpu_phase.is_even();
        self.pulse1.clock(&frame_clock, apu_tick);
        self.pulse2.clock(&frame_clock, apu_tick);
        self.noise.clock(&frame_clock, apu_tick);
        self.triangle.clock(&frame_clock, true);

        // Handle consecutive IRQ reassert quirk
        if self.frame_irq_disable {
            self.frame_irq_reassert = 0; // If disabled, stop reasserts
        } else if self.frame_irq_reassert != 0 {
            self.frame_irq_reassert -= 1;
            self.frame_irq_rising = true;
            self.status_register
                .insert(ApuStatusRegister::FRAME_INTERRUPT);
        }

        self.clock_apu_output();
        self.cpu_phase.toggle();
        self.seq_phase.toggle();
    }

    fn clock_apu_output(&mut self) {
        let sample = self.sample();
        let dac = (sample * DAC_SCALE * BLIP_GAIN).round() as i32;

        let delta = dac - self.last_dac;
        if delta != 0 {
            self.output.add_delta(delta);
            self.last_dac = dac;
        }

        self.output.step_cpu_cycle();
    }

    fn sample(&self) -> f32 {
        let pulse1 = if self.mute_pulse1 {
            0.0
        } else {
            self.pulse1.sample() as f32
        };
        let pulse2 = if self.mute_pulse2 {
            0.0
        } else {
            self.pulse2.sample() as f32
        };
        let triangle = if self.mute_triangle {
            0.0
        } else {
            self.triangle.sample() as f32
        };
        let noise = if self.mute_noise {
            0.0
        } else {
            self.noise.sample() as f32
        };
        let dmc = if self.mute_dmc { 0.0 } else { 0.0 }; // TODO

        let mut sample;
        #[cfg(feature = "linear-apu-approximation")]
        {
            // See linear approximation on: https://www.nesdev.org/wiki/APU_Mixer
            let pulse = pulse1 + pulse2;
            let tnd = 0.00851 * triangle + 0.00494 * noise + 0.00335 * dmc;
            sample = 0.00752 * pulse + tnd;
        }
        #[cfg(not(feature = "linear-apu-approximation"))]
        {
            // See: https://www.nesdev.org/wiki/APU_Mixer
            // pulse_out = 95.88 / (8128.0 / (pulse1 + pulse2) + 100.0);
            // tnd_out   = 159.79 / (1.0 / (triangle/8227.0 + noise/12241.0 + dmc/22638.0) + 100.0);
            // output    = pulse_out + tnd_out;
            let pulse_sum = pulse1 + pulse2;
            let pulse_out = if pulse_sum > 0.0 {
                95.88 / (8128.0 / (pulse1 + pulse2) + 100.0)
            } else {
                0.0
            };

            let tnd_sum = triangle / 8227.0 + noise / 12241.0 + dmc / 22638.0;
            let tnd_out = if tnd_sum > 0.0 {
                159.79 / (1.0 / tnd_sum + 100.0)
            } else {
                0.0
            };
            sample = pulse_out + tnd_out;
        }

        // sample = self
        //     .high_pass_90
        //     .high_pass(sample, 90.0, self.sample_rate as f32);
        // sample = self
        //     .high_pass_440
        //     .high_pass(sample, 440.0, self.sample_rate as f32);
        // sample = self
        //     .low_pass_14k
        //     .low_pass(sample, 14_000.0, self.sample_rate as f32);
        sample
    }

    // #[inline(always)]
    // pub fn get_last_sample(&self) -> f32 {
    //     self.current_sample_raw
    // }

    // pub fn filter_raw_sample(&mut self, raw_sample: f32) -> f32 {
    //     let mut sample = raw_sample;
    //     sample = self
    //         .high_pass_90
    //         .high_pass(sample, 90.0, self.sample_rate as f32);
    //     sample = self
    //         .high_pass_440
    //         .high_pass(sample, 440.0, self.sample_rate as f32);
    //     sample = self
    //         .low_pass_14k
    //         .low_pass(sample, 14_000.0, self.sample_rate as f32);
    //     sample
    // }

    #[inline(always)]
    pub fn irq_line(&self) -> bool {
        let frame_interrupt = self
            .status_register
            .contains(ApuStatusRegister::FRAME_INTERRUPT)
            && !self.frame_irq_disable;

        let dmc_interrupt = self
            .status_register
            .contains(ApuStatusRegister::DMC_INTERRUPT);

        frame_interrupt || dmc_interrupt
    }

    pub fn end_frame(&mut self, cpu_cycles: u32) {
        self.output.end_frame(cpu_cycles);
    }

    pub fn samples_available(&self) -> usize {
        self.output.samples_available()
    }

    pub fn read_samples_f32(&mut self, out: &mut [f32]) -> usize {
        self.output.read_samples_f32(out)
    }

    /// CPU cycles needed to generate `N` more samples at current rates
    pub fn clocks_needed(&self, sample_count: u32) -> u32 {
        self.output.clocks_needed(sample_count)
    }
}
