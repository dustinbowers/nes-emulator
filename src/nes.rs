use crate::bus::Bus;
use crate::cartridge::Cartridge;
use crate::controller::joypad::Joypad;
use crate::cpu::processor::CPU;
use crate::ppu::PPU;
use crate::rom::Rom;

pub struct NES {
    pub cpu: CPU,
    pub ppu: PPU,
    pub bus: Bus,
}

impl NES {
    pub fn new(mut cpu: CPU, mut ppu: PPU, cartridge: Box<dyn Cartridge>) -> Self {
        let bus = Bus::new(cartridge);
        Self { cpu, ppu, bus }
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