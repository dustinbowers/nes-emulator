mod dmc_channel;
mod noise_channel;
mod pulse_channel;
mod triangle_channel;
mod units;

use crate::apu::noise_channel::NoiseChannel;
use crate::apu::pulse_channel::PulseChannel;
use crate::apu::triangle_channel::TriangleChannel;
use crate::ppu::PpuBusInterface;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

enum MasterSequenceMode {
    FourStep,
    FiveStep,
}
const SAMPLE_RATE: f32 = 44_100.0;

pub trait ApuBusInterface {
    fn apu_bus_read(&mut self, addr: u16) -> u8;
    fn irq(&mut self);
}

/*
   mode 0:    mode 1:       function
   ---------  -----------  -----------------------------
    - - - f    - - - - -    IRQ (if bit 6 is clear)
    - l - l    - l - - l    Length counter and sweep
    e e e e    e e e - e    Envelope and linear counter
*/
pub struct APU {
    bus: Option<*mut dyn ApuBusInterface>,

    pub pulse1: PulseChannel,
    pub pulse2: PulseChannel,
    pub triangle: TriangleChannel,
    pub noise: NoiseChannel,

    pub enable_dmc: bool,
    pub enable_noise: bool,
    pub enable_triangle: bool,
    pub enable_pulse2: bool,
    pub enable_pulse1: bool,

    pub master_sequence_mode: bool,
    pub irq_disable: bool,
    pub frame_clock_counter: u8,
    pub clock_counter: u32,
}

impl APU {
    pub fn new() -> APU {
        APU {
            bus: None,
            pulse1: PulseChannel::new(true),
            pulse2: PulseChannel::new(false),
            triangle: TriangleChannel::new(),
            noise: NoiseChannel::new(),
            enable_dmc: false,
            enable_noise: false,
            enable_triangle: false,
            enable_pulse2: false,
            enable_pulse1: false,
            irq_disable: false,

            master_sequence_mode: false,
            frame_clock_counter: 0,
            clock_counter: 0,
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

            0x4010..=0x4013 => { // DMC
            }
            0x4015 => {
                // Control / Status
                self.enable_dmc = value & 1 << 4 != 0;
                self.enable_noise = value & 1 << 3 != 0;
                self.enable_triangle = value & 1 << 2 != 0;
                self.enable_pulse2 = value & 1 << 1 != 0;
                self.enable_pulse1 = value & 1 << 0 != 0;
            }
            0x4017 => {
                // Frame Counter
                /*
                   0x4017: MI-- ----
                       M: Mode.- bit 7
                       I: IRQ Off - bit 6
                */
                self.master_sequence_mode = value & 0b1000_0000 != 0;
                self.irq_disable = value & 0b0100_0000 != 0;
                self.frame_clock_counter = 0;

                // When bit 7 is set, reset frameclock counter and clock all channels
                if self.master_sequence_mode == true {
                    self.pulse1.clock(true, true);
                    self.pulse2.clock(true, true);
                    self.triangle.clock(true);
                }
            }
            _ => {
                panic!("API write to invalid register! {:04x}", addr);
            }
        }
    }

    pub fn clock(&mut self, cpu_cycles: usize) {
        let mut quarter_frame_clock = false;
        let mut half_frame_clock = false;

        if cpu_cycles % 2 == 0 {
            self.clock_counter += 1;
            match self.master_sequence_mode {
                false => {
                    // 4-step
                    match self.clock_counter {
                        3729 => {
                            quarter_frame_clock = true;
                        }
                        7457 => {
                            quarter_frame_clock = true;
                            half_frame_clock = true;
                        }
                        11186 => {
                            quarter_frame_clock = true;
                        }
                        14915 => {
                            quarter_frame_clock = true;
                            half_frame_clock = true;
                            self.clock_counter = 0;

                            if self.irq_disable == false {
                                if let Some(bus_ptr) = self.bus {
                                    unsafe {
                                        (*bus_ptr).irq();
                                    }
                                }
                            }
                        }
                        _ => {}
                    };
                }
                true => {
                    // 5-step
                    match self.clock_counter {
                        3729 => {
                            quarter_frame_clock = true;
                        }
                        7457 => {
                            quarter_frame_clock = true;
                            half_frame_clock = true;
                        }
                        11186 => {
                            quarter_frame_clock = true;
                        }
                        18640 => {
                            quarter_frame_clock = true;
                            half_frame_clock = true;
                            self.clock_counter = 0;
                        }
                        _ => {}
                    };
                }
            }

            self.pulse1.clock(quarter_frame_clock, half_frame_clock);
            self.pulse2.clock(quarter_frame_clock, half_frame_clock);
            self.noise.clock(quarter_frame_clock, half_frame_clock);
        }
        self.triangle.clock(quarter_frame_clock);
    }

    pub fn sample(&self) -> f32 {
        let pulse1 = self.pulse1.sample() as f32;
        let pulse2 = self.pulse2.sample() as f32;
        let triangle = self.triangle.sample() as f32;
        let noise = self.noise.sample() as f32;
        let dmc = 0.0; // TODO

        // pulse_out = 95.88 / (8128.0 / (pulse1 + pulse2) + 100.0);
        // tnd_out   = 159.79 / (1.0 / (triangle/8227.0 + noise/12241.0 + dmc/22638.0) + 100.0);
        // output    = pulse_out + tnd_out;
        let pulse_out = 95.88 / (8128.0 / (pulse1 + pulse2) + 100.0);
        let tnd_out =
            159.79 / (1.0 / (triangle / 8227.0 + noise / 12241.0 + dmc / 22638.0) + 100.0);
        let output = pulse_out + tnd_out;
        output
    }

    fn clock_irq(&mut self) {}
}
