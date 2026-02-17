use super::units::sequence_timer::SequenceTimer;
use crate::nes::apu::units::dmc_output::DmcOutput;

const RATE_TABLE: [u16; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
];

pub struct DmcChannel {
    seq_timer: SequenceTimer,
    output: DmcOutput,

    enabled: bool,

    irq_enabled: bool,
    irq_pending: bool,
    loop_flag: bool,

    sample_address: u16,
    current_address: u16,

    sample_length: u16,
    bytes_remaining: u16,

    sample_buffer: Option<u8>,
}

impl DmcChannel {
    pub fn new() -> DmcChannel {
        DmcChannel {
            seq_timer: SequenceTimer::new(),
            output: DmcOutput::new(),

            enabled: false,

            irq_enabled: false,
            irq_pending: false,
            loop_flag: false,

            sample_address: 0,
            sample_length: 0,

            current_address: 0,
            bytes_remaining: 0,

            sample_buffer: None,
        }
    }

    pub fn write_4010(&mut self, value: u8) {
        /* $4010:       IL--.RRRR (write)
              bit 7    I---.---- IRQ enabled flag
              bit 6    -L--.---- Loop flag
              bits 3-0 ----.RRRR Rate index
        */
        let new_irq_enabled = value & 0x80 != 0;
        if !new_irq_enabled {
            self.irq_pending = false;
        }
        self.irq_enabled = new_irq_enabled;

        self.loop_flag = value & 0x40 != 0;
        let rate_index = (value & 0x0F) as usize;
        let period = RATE_TABLE[rate_index];

        self.seq_timer.set_reload(period - 1);
    }

    pub fn write_4011(&mut self, value: u8) {
        /* $4011:       -DDD.DDDD Direct load (write)
              bits 6-0	-DDD.DDDD The DMC output level is set to D, an unsigned value.
        */
        self.output.direct_load(value & 0x7F);
    }

    pub fn write_4012(&mut self, value: u8) {
        // $4012    AAAA.AAAA address (write)
        self.sample_address = 0xC000 + (value as u16 * 64);
    }

    pub fn write_4013(&mut self, value: u8) {
        // $4013    LLLL.LLLL Sample length (write)
        self.sample_length = (value as u16 * 16) + 1;
    }
}

impl DmcChannel {
    pub fn start(&mut self) {
        self.current_address = self.sample_address;
        self.bytes_remaining = self.sample_length;
    }

    pub fn write_4015(&mut self, value: u8) {
        // All writes clear DMC interrupt
        self.irq_pending = false;

        let enable = (value & 0x10) != 0;

        if !enable {
            self.bytes_remaining = 0;
            self.enabled = false;
            return;
        }

        self.enabled = true;

        // Restart if bytes_remaining is zero
        if self.bytes_remaining == 0 {
            self.current_address = self.sample_address;
            self.bytes_remaining = self.sample_length;
            // sample_buffer and shift register remain intact
            // so that it can finish playing before looping
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        // If the DMC bit is clear, the DMC bytes remaining will be set to 0 and the
        // DMC will silence when it empties.
        if self.enabled && !enabled {
            self.bytes_remaining = 0;
        }
        // If the DMC bit is set, the DMC sample will be restarted only if its bytes
        // remaining is 0. If there are bits remaining in the 1-byte sample buffer,
        // these will finish playing before the next sample is fetched.
        else if !self.enabled && enabled {
            if self.bytes_remaining == 0 {
                self.start();
            }
        }
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.bytes_remaining > 0
    }

    pub fn clear_irq(&mut self) {
        self.irq_pending = false;
    }
    pub fn irq_pending(&self) -> bool {
        self.irq_pending
    }

    pub fn wants_bus(&self) -> bool {
        self.enabled && self.sample_buffer.is_none() && self.bytes_remaining > 0
    }

    pub fn dma_addr(&self) -> u16 {
        self.current_address
    }

    pub fn supply_dma_byte(&mut self, byte: u8) {
        self.sample_buffer = Some(byte);

        self.current_address = if self.current_address == 0xFFFF {
            0x8000
        } else {
            self.current_address.wrapping_add(1)
        };

        self.bytes_remaining = self.bytes_remaining.saturating_sub(1);

        if self.bytes_remaining == 0 {
            if self.loop_flag {
                self.current_address = self.sample_address;
                self.bytes_remaining = self.sample_length;
            } else if self.irq_enabled {
                self.irq_pending = true;
            }
        }
    }

    pub fn clock(&mut self) {
        if self.seq_timer.clock() {
            self.seq_timer.reset();

            let shift_empty = self.output.clock();
            if shift_empty {
                if let Some(byte) = self.sample_buffer.take() {
                    self.output.load_shift_register(byte);
                }
            }
        }
    }

    pub fn sample(&self) -> u8 {
        self.output.level()
    }
}
