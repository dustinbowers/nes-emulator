use super::units::sequence_timer::SequenceTimer;

pub struct DmcChannel {
    seq_timer: SequenceTimer,

    irq_enabled: bool,
    loop_flag: bool,
    // output: DmcOutput,

    // rate: i16,
    register: u8,
    level: u8,
    current_bit: u8,

    sample_address: u16,
    current_address: u16,

    sample_length: u8,
    bytes_remaining: u8,
}

impl DmcChannel {
    pub fn new() -> DmcChannel {
        DmcChannel {
            seq_timer: SequenceTimer::new(),
            irq_enabled: false,
            loop_flag: false,
            // output: DmcOutput::new(),
            register: 0,
            level: 0,
            current_bit: 0,

            // rate: 0,
            sample_address: 0,
            sample_length: 0,
            current_address: 0,
            bytes_remaining: 0,
        }
    }

    pub fn write_4010(&mut self, value: u8) {
        /* $4010:       IL--.RRRR (write)
              bit 7    I---.---- IRQ enabled flag
              bit 6    -L--.---- Loop flag
              bits 3-0 ----.RRRR Rate index
        */
        self.irq_enabled = value & 0b1000_0000 != 0;
        self.loop_flag = value & 0b0100_0000 != 0;

        let rate_index = value & 0b0000_1111;
        const RATE_TABLE: [u16; 16] = [
            428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
        ];
        // self.rate = RATE_TABLE[rate_index as usize];
        self.seq_timer.set_reload(RATE_TABLE[rate_index as usize]);
    }

    pub fn write_4011(&mut self, value: u8) {
        /* $4011:       -DDD.DDDD Direct load (write)
              bits 6-0	-DDD.DDDD The DMC output level is set to D, an unsigned value.
        */
        // self.output.direct_load(value & 0b0111_1111);
        self.level = value & 0b0111_1111;
    }

    pub fn write_4012(&mut self, value: u8) {
        // $4012    AAAA.AAAA address (write)
        self.sample_address = 0xC000 + (value as u16 * 64);
    }

    pub fn write_4013(&mut self, value: u8) {
        // $4013    LLLL.LLLL Sample length (write)
        self.sample_length = (value * 16) + 1;
    }
}

impl DmcChannel {
    pub fn start(&mut self) {
        self.current_address = self.sample_address;
        self.bytes_remaining = self.sample_length;
    }

    pub fn disable(&mut self) {
        // TODO
    }

    pub fn clock(&mut self) {
        let advance_output = self.seq_timer.clock();
        if self.bytes_remaining > 0 {
            self.seq_timer.reset();
        }

        if advance_output {
            self.current_bit += 1;
            if self.current_bit >= 8 {
                // TODO: pause CPU and load another byte
                self.current_bit = 0;
                self.bytes_remaining -= 1;

                // Loop if enabled
                if self.bytes_remaining == 0 && self.loop_flag {
                    self.bytes_remaining = self.sample_length;
                    self.current_address = self.sample_address;
                }
            }

            let level_up = (self.register >> self.current_bit) & 0b1 != 0;
            if self.level >= 2 && self.level <= 125 {
                if level_up {
                    self.level += 2;
                } else {
                    self.level -= 2;
                }
            }
        }
    }

    pub fn sample(&self) -> u8 {
        self.level
    }
}
