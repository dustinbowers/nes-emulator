use crate::bus::Bus;
use crate::cartridge::Cartridge;

pub struct NES {
    pub bus: &'static mut Bus,
}

impl NES {
    pub fn new(cartridge: Box<dyn Cartridge>) -> Self {
        let mut bus = Bus::new(cartridge);
        Self { bus }
    }

    pub fn tick(&mut self) -> bool {
        // CPU
        let (_, _, is_breaking) = self.bus.cpu.tick();

        // PPU ticks 3 times per CPU cycle
        let mut frame_ready = false;
        for _ in 0..3 {
            if self.bus.ppu.tick() {
                frame_ready = true;
            }
        }

        // TODO: APU

        // if frame_ready {
        //     panic!("frame_ready!");
        // }
        frame_ready
    }

    pub fn get_frame_buffer(&self) -> &[u8; 256 * 240] {
        return &self.bus.ppu.frame_buffer;
    }
}
