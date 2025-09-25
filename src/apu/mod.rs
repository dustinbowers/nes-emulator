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

            0x4008..=0x400B => { // Triangle
            }
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

                // Writing to $4017 with bit 7 set ($80) will immediately clock all
                // of its controlled units at the beginning of the 5-step sequence
                // if self.master_sequence_mode {
                //     self.clock_length_counter_and_sweep();
                //     self.clock_env_and_linear_counter();
                // }
            }
            _ => {
                panic!("API write to invalid register!")
            }
        }
    }

    // pub fn tick(&mut self) {
    //     let mut buf = self.buffer.lock().unwrap();
    //     // println!("apu buf len: {}", buf.len());
    //     for i in 0..5 {
    //         if buf.len() < 8192 {
    //             let sample = self.sq.sample();
    //             buf.push_back(sample);
    //         }
    //     }
    //
    // }

    pub fn clock(&mut self, cpu_cycles: usize) {
        if cpu_cycles % 2 == 0 {
            self.clock_counter += 1;

            let mut quarter_frame_clock = false;
            let mut half_frame_clock = false;
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
        }

        self.triangle.clock();
    }

    pub fn sample(&self) -> u8 {
        let out = self.pulse1.sample();
        out
    }

    // pub fn run_frame_counter(&mut self) {
    //     let actions = if self.master_sequence_mode == false {
    //         self.frame_clock_counter = if self.frame_clock_counter > 3 { 0 } else { self.frame_clock_counter + 1};
    //         MODE_1[(self.frame_clock_counter % 4) as usize]
    //     } else {
    //         self.frame_clock_counter = if self.frame_clock_counter > 4 { 0 } else { self.frame_clock_counter + 1};
    //         MODE_2[(self.frame_clock_counter % 5) as usize]
    //     };
    //
    //     if actions[0] == 'F' { // IRQ (if bit 6 set)
    //         if self.irq_disable == false {
    //
    //         }
    //     }
    //     if actions[1] == 'L' {
    //         // length-counter and sweep
    //         self.clock_length_counter_and_sweep();
    //     }
    //     if actions[2] == 'C' { // envelope and linear counter
    //     }
    //
    // }

    fn clock_irq(&mut self) {}

    // fn clock_env_and_linear_counter(&mut self) {
    //
    // }

    // fn clock_length_counter_and_sweep(&mut self) {
    //
    //
    // }
}
