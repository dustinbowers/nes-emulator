mod pulse_channel;
mod registers;
mod square_wave;
mod triangle_channel;
mod units;

use crate::apu::pulse_channel::PulseChannel;
use crate::apu::triangle_channel::TriangleChannel;
use square_wave::SquareWave;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

enum MasterSequenceMode {
    FourStep,
    FiveStep,
}
const SAMPLE_RATE: f32 = 44_100.0;

/*
   mode 0:    mode 1:       function
   ---------  -----------  -----------------------------
    - - - f    - - - - -    IRQ (if bit 6 is clear)
    - l - l    - l - - l    Length counter and sweep
    e e e e    e e e - e    Envelope and linear counter
*/
// const MODE_1: [[char; 3]; 4] = [
//     ['-', '-', 'C'],
//     ['-', 'L', 'C'],
//     ['-', '-', 'C'],
//     ['F', 'L', 'C'],
// ];
// const MODE_2: [[char; 3]; 5] = [
//     ['-', '-', 'C'],
//     ['-', 'L', 'C'],
//     ['-', '-', 'C'],
//     ['-', '-', '-'],
//     ['-', 'L', 'C'],
// ];
pub struct APU {
    pub buffer: Arc<Mutex<VecDeque<i16>>>,
    pub triangle: TriangleChannel,
    pub sq: SquareWave,

    pub pulse1: PulseChannel,
    pub pulse2: PulseChannel,

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
            buffer: Arc::new(Mutex::new(VecDeque::<i16>::with_capacity(8192))),
            triangle: TriangleChannel::new(),
            sq: SquareWave::new(440.0),
            pulse1: PulseChannel::new(true),
            pulse2: PulseChannel::new(false),
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

    pub fn get_audio_buffer(&mut self) -> Arc<Mutex<VecDeque<i16>>> {
        return self.buffer.clone();
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
            0x400a => self.triangle.write_400a(value),
            0x400b => self.triangle.write_400b(value),

            0x400C..=0x400F => { // Noise
            }
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


            }
            _ => {
                panic!("API write to invalid register!")
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
                            // TODO: trigger IRQ if enabled
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

        }
        self.triangle.clock(quarter_frame_clock);
    }

    pub fn sample(&self) -> f32 {
        let pulse1 = self.pulse1.sample() as f32;
        let pulse2 = self.pulse2.sample() as f32;
        let triangle = self.triangle.sample() as f32;
        let noise = 0.0;
        let dmc = 0.0;


        // pulse_out = 95.88 / (8128.0 / (pulse1 + pulse2) + 100.0);
        // tnd_out   = 159.79 / (1.0 / (triangle/8227.0 + noise/12241.0 + dmc/22638.0) + 100.0);
        // output    = pulse_out + tnd_out;
        let pulse_out = 95.88 / (8128.0 / (pulse1 + pulse2) + 100.0);
        let tnd_out = 0.0;
        let output = pulse_out + tnd_out;

        output
    }

    fn clock_irq(&mut self) {}
}
