use dmc_channel::DmcChannel;
use noise_channel::NoiseChannel;
use pulse_channel::PulseChannel;
use thiserror::Error;
use triangle_channel::TriangleChannel;

mod dmc_channel;
mod noise_channel;
mod pulse_channel;
mod triangle_channel;
mod units;

pub trait ApuBusInterface {
    fn apu_bus_read(&mut self, addr: u16) -> u8;
    fn irq(&mut self);
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
pub struct APU {
    bus: Option<*mut dyn ApuBusInterface>,

    pub pulse1: PulseChannel,
    pub pulse2: PulseChannel,
    pub triangle: TriangleChannel,
    pub noise: NoiseChannel,
    pub dmc: DmcChannel,

    pub mute_pulse1: bool,
    pub mute_pulse2: bool,
    pub mute_triangle: bool,
    pub mute_noise: bool,
    pub mute_dmc: bool,

    pub enable_dmc: bool,
    pub enable_noise: bool,
    pub enable_triangle: bool,
    pub enable_pulse2: bool,
    pub enable_pulse1: bool,

    pub master_sequence_mode: bool,
    pub frame_clock_counter: u8,
    pub clock_counter: u32,

    pub irq_disable: bool,
    pub dmc_interrupt: bool,
    pub frame_interrupt: bool,

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
            pulse1: PulseChannel::new(true),
            pulse2: PulseChannel::new(false),
            triangle: TriangleChannel::new(),
            noise: NoiseChannel::new(),
            dmc: DmcChannel::new(),

            mute_pulse1: false,
            mute_pulse2: false,
            mute_triangle: false,
            mute_noise: false,
            mute_dmc: false,

            enable_dmc: false,
            enable_noise: false,
            enable_triangle: false,
            enable_pulse2: false,
            enable_pulse1: false,

            master_sequence_mode: false,
            frame_clock_counter: 0,
            clock_counter: 0,

            irq_disable: false,
            dmc_interrupt: false,
            frame_interrupt: false,

            error: None,
        }
    }

    pub fn reset(&mut self) {
        self.pulse1 = PulseChannel::new(true);
        self.pulse2 = PulseChannel::new(false);
        self.triangle = TriangleChannel::new();
        self.noise = NoiseChannel::new();
        self.dmc = DmcChannel::new();

        self.mute_pulse1 = false;
        self.mute_pulse2 = false;
        self.mute_triangle = false;
        self.mute_noise = false;
        self.mute_dmc = false;

        self.enable_dmc = false;
        self.enable_noise = false;
        self.enable_triangle = false;
        self.enable_pulse2 = false;
        self.enable_pulse1 = false;

        self.master_sequence_mode = false;
        self.frame_clock_counter = 0;
        self.clock_counter = 0;

        self.irq_disable = false;
        self.dmc_interrupt = false;
        self.frame_interrupt = false;

        self.error = None;
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        println!("APU::read({:04X})", addr);
        match addr {
            0x4015 => {
                let mut output = 0;
                output |= if self.dmc_interrupt { 0b1000_0000 } else { 0 };
                output |= if self.frame_interrupt { 0b0100_0000 } else { 0 };
                // bit 5 skipped
                output |= if 1 == 0 { 0b0001_0000 } else { 0 };
                output |= if self.noise.is_enabled() {
                    0b0000_1000
                } else {
                    0
                };
                output |= if self.triangle.is_enabled() {
                    0b0000_0100
                } else {
                    0
                };
                output |= if self.pulse2.is_enabled() {
                    0b0000_0010
                } else {
                    0
                };
                output |= if self.pulse1.is_enabled() {
                    0b0000_0001
                } else {
                    0
                };
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
                // Control / Status
                self.enable_dmc = value & 1 << 4 != 0;
                self.enable_noise = value & 1 << 3 != 0;
                self.enable_triangle = value & 1 << 2 != 0;
                self.enable_pulse2 = value & 1 << 1 != 0;
                self.enable_pulse1 = value & 1 << 0 != 0;
                // println!("APU::write({:04X}, {:08b})", addr, value);

                if !self.enable_pulse1 {
                    self.pulse1.disable();
                }
                if !self.enable_pulse2 {
                    self.pulse2.disable();
                }
                if !self.enable_triangle {
                    self.triangle.disable();
                }
                if !self.enable_noise {
                    self.noise.disable();
                }
                if !self.enable_dmc {
                    self.dmc.disable();
                }
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
                self.error = Some(ApuError::InvalidRegisterWrite(addr));
            }
        }
    }

    pub fn clock(&mut self, cpu_cycles: usize) {
        let mut quarter_frame_clock = false;
        let mut half_frame_clock = false;

        if cpu_cycles.is_multiple_of(2) {
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

        // pulse_out = 95.88 / (8128.0 / (pulse1 + pulse2) + 100.0);
        // tnd_out   = 159.79 / (1.0 / (triangle/8227.0 + noise/12241.0 + dmc/22638.0) + 100.0);
        // output    = pulse_out + tnd_out;
        let pulse_out = 95.88 / (8128.0 / (pulse1 + pulse2) + 100.0);
        let tnd_out =
            159.79 / (1.0 / (triangle / 8227.0 + noise / 12241.0 + dmc / 22638.0) + 100.0);

        pulse_out + tnd_out
    }

    fn clock_irq(&mut self) {}
}
