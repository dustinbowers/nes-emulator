use crate::bus::Bus;
use crate::cartridge::Cartridge;

const PPU_WARMUP_CYCLES: usize = 29781;

pub struct NES {
    pub bus: &'static mut Bus,
    pub ppu_warmed_up: bool,
    pub cpu_cycles: usize,

}

impl NES {
    pub fn new(cartridge: Box<dyn Cartridge>) -> Self {
        let mut bus = Bus::new(cartridge);
        Self {
            bus,
            cpu_cycles: 0,
            ppu_warmed_up: false,
        }
    }

    pub fn tick(&mut self) -> bool {
        // CPU
        self.bus.cpu.tick();
        self.cpu_cycles += 1;
        if self.cpu_cycles > 1_000_000 {
            self.cpu_cycles -= 1_000_000;
        }

        // PPU ticks 3 times per CPU cycle
        let mut frame_ready = false;

        if self.ppu_warmed_up {
            for _ in 0..3 {
                if self.bus.ppu.tick() {
                    frame_ready = true;
                }
            }
        } else if self.cpu_cycles > PPU_WARMUP_CYCLES {
            println!("=== PPU WARMED UP at {} cpu_cycles cycles", self.cpu_cycles);
            self.ppu_warmed_up = true;
        }

        // TODO: APU

        frame_ready
    }

    pub fn get_frame_buffer(&self) -> &[u8; 256 * 240] {
        return &self.bus.ppu.frame_buffer;
    }
    pub fn clear_frame(&mut self) {
        self.bus.ppu.frame_buffer.fill(0);
    }
}
