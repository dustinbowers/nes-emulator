use crate::bus::Bus;
use crate::cartridge::Cartridge;

pub struct NES {
    pub bus: Bus,
}

impl NES {
    pub fn new(cartridge: Box<dyn Cartridge>) -> Self {
        let bus = Bus::new(cartridge);
        Self { bus }
    }

    pub fn tick(&mut self) -> bool {
        // CPU
        let (_, _, is_breaking) = self.bus.cpu.tick();

        // PPU
        self.bus.ppu.tick();
        self.bus.ppu.tick();
        self.bus.ppu.tick();

        // TODO: APU

        is_breaking
    }
}
