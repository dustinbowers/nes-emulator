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
        println!("NES::tick()");
        // CPU
        let (_, _, is_breaking) = self.bus.cpu.tick();

        // PPU
        println!("NES::tick - starting ppu ticks");
        self.bus.ppu.tick();
        self.bus.ppu.tick();
        self.bus.ppu.tick();

        // TODO: APU

        is_breaking
    }
}
