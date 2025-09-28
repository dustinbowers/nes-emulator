use crate::apu::units::dmc_output::DmcOutput;
use crate::apu::units::sequence_timer::SequenceTimer;

pub struct DmcChannel {
    seq_timer: SequenceTimer,

    irq_enabled: bool,
    loop_flag: bool,
    output: DmcOutput,

    rate: i16,

    direct_load: u8,
    sample_address: u16,
    sample_length: u8,
    current_address: u16,
    bytes_remaining: u8,
}

impl DmcChannel {
    pub fn new() -> DmcChannel {
        DmcChannel {
            seq_timer: SequenceTimer::new(),
            irq_enabled: false,
            loop_flag: false,
            output: DmcOutput::new(),
            rate: 0,
            direct_load: 0,
            sample_address: 0,
            sample_length: 0,
            current_address: 0,
            bytes_remaining: 0,
        }
    }

    pub fn write_4010(&mut self, value: u8) {
        self.irq_enabled = value & 0b1000_0000 != 0;
        self.loop_flag = value & 0b0100_0000 != 0;

        let rate_index = value & 0b0000_1111;
        const RATE_TABLE: [i16; 16] = [
            428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
        ];
        self.rate = RATE_TABLE[rate_index as usize];
    }

    pub fn write_4011(&mut self, value: u8) {
        self.output.set_register(value & 0b0111_1111);
    }

    pub fn write_4012(&mut self, value: u8) {
        self.sample_address = 0xC000 + (value as u16 * 64);
    }

    pub fn write_4013(&mut self, value: u8) {
        self.sample_length = (value * 16) + 1;
    }
}

impl DmcChannel {
    pub fn start(&mut self) {
        self.current_address = self.sample_address;
        self.bytes_remaining = self.sample_length;
    }
    pub fn clock(&mut self) {}
    pub fn sample(&self) -> u8 {
        0
    }
}
