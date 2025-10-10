use super::units::envelope::Envelope;
use super::units::length_counter::LengthCounter;
use super::units::sequence_timer::SequenceTimer;

pub enum NoiseMode {
    Long,
    Short,
}

pub struct NoiseChannel {
    pub seq_timer: SequenceTimer,
    pub length_counter: LengthCounter,
    pub envelope: Envelope,
    mode: NoiseMode,

    shifter: u16,
}

impl NoiseChannel {
    pub fn new() -> NoiseChannel {
        NoiseChannel {
            seq_timer: SequenceTimer::new(),
            length_counter: LengthCounter::new(),
            envelope: Envelope::new(),
            mode: NoiseMode::Long,

            shifter: 0x7FFF,
        }
    }

    pub fn write_400c(&mut self, value: u8) {
        let length_counter_halt = value & 0b0010_0000 != 0;
        self.envelope.set(value);
        self.length_counter.set_halt(length_counter_halt);
    }

    pub fn write_400e(&mut self, value: u8) {
        self.mode = match value & 0b1000_0000 == 0 {
            true => NoiseMode::Long,   // Flag clear == 32767 steps
            false => NoiseMode::Short, // Flag set == 93 or 91 steps
        };

        let timer_period = value & 0b0000_1111;
        const NOISE_TABLE: [u16; 16] = [
            4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
        ];
        self.seq_timer
            .set_reload(NOISE_TABLE[timer_period as usize]);
    }

    pub fn write_400f(&mut self, value: u8) {
        let length_counter_load = value >> 3;
        self.length_counter.set(length_counter_load);
        self.envelope.start();
    }
}

impl NoiseChannel {
    pub fn disable(&mut self) {
        self.length_counter.set_enabled(false);
        self.length_counter.set_halt(true);
    }

    pub fn is_enabled(&self) -> bool {
        self.length_counter.output() > 0
    }

    fn shift_noise(&mut self) {
        // Linear feedback shifter
        let feedback = match self.mode {
            NoiseMode::Long => {
                let a = self.shifter & 0b1;
                let b = (self.shifter >> 1) & 0b1;
                a ^ b
            }
            NoiseMode::Short => {
                let a = self.shifter & 0b1;
                let b = (self.shifter >> 6) & 0b1;
                a ^ b
            }
        };
        self.shifter = (feedback << 14) | (self.shifter >> 1);
    }

    pub fn clock(&mut self, quarter_frame_clock: bool, half_frame_clock: bool) {
        if quarter_frame_clock {
            self.envelope.clock();
        }

        if half_frame_clock {
            self.length_counter.clock();
        }

        if self.seq_timer.clock() {
            self.shift_noise();
        }
    }

    pub fn sample(&self) -> u8 {
        if self.length_counter.output() == 0 || (self.shifter & 0b1) != 0 {
            0
        } else {
            self.envelope.output()
        }
    }
}
