use cpu::CpuBusInterface;
pub mod apu;
pub mod bus;
pub mod cartridge;
pub mod controller;
pub mod cpu;
pub mod ppu;
pub mod tracer;

#[cfg(feature = "testing-utils")]
pub mod test_utils;

// use super::trace;
use crate::trace;
use bus::nes_bus::NesBus;
use cartridge::Cartridge;
use cartridge::rom::{Rom, RomError};

pub const PPU_HZ: u64 = 5_369_318;
pub const CPU_HZ_NTSC: f64 = PPU_HZ as f64 / 3.0;

const OAM_DMA_START_CYCLES: usize = 0;
const OAM_DMA_DONE_CYCLES: usize = 512;

enum DmaMode {
    Oam,
    None,
}

pub enum RunState {
    Running,
    Paused,
}

// SAFETY:
// NES is strictly single-threaded.
// Expected use is created on the main thread and moved exactly once
// into the audio thread, which owns it exclusively.
// No other thread ever accesses NES or its internal raw pointers.
unsafe impl Send for NES {}

pub struct NES {
    pub run_state: RunState,
    pub bus: &'static mut NesBus,
    master_clock: u64,
    dma_mode: DmaMode,

    pub oam_transfer_cycles: usize,

    // pub cycle_acc: f64,
    pub ppu_remainder: u64,
    pub last_apu_sample_raw: f32,
}

impl Default for NES {
    fn default() -> Self {
        Self::new()
    }
}

impl NES {
    pub fn new() -> Self {
        let bus = NesBus::new();
        Self {
            run_state: RunState::Running,
            bus,
            master_clock: 0,
            dma_mode: DmaMode::None,
            oam_transfer_cycles: 0,

            // cycle_acc: 0.0,
            ppu_remainder: 0,
            last_apu_sample_raw: 0.0,
        }
    }

    #[allow(dead_code)]
    pub fn new_with_cartridge(cartridge: Box<dyn Cartridge>) -> Self {
        let mut nes = NES::new();
        nes.insert_cartridge(cartridge);
        nes
    }

    pub fn insert_cartridge(&mut self, cartridge: Box<dyn Cartridge>) {
        self.bus.insert_cartridge(cartridge);
    }

    pub fn parse_rom_bytes(rom_bytes: &Vec<u8>) -> Result<Box<dyn Cartridge>, RomError> {
        let rom = Rom::parse(rom_bytes)?;
        let cart = rom.into_cartridge()?;
        Ok(cart)
    }

    /// Ticks emulator once
    ///
    /// Ticks the NES emulator forward by 1 PPU cycle
    ///
    /// # Returns
    ///
    /// Returns a `(bool, bool)` tuple
    /// - First value is `true` if this tick triggered a CPU tick, and `false` otherwise
    /// - Second value is `true` if a new frame is ready to be rendered, and `false` otherwise
    pub fn tick(&mut self) -> (bool, bool) {
        // CPU runs at 1/3 PPU speed
        let mut cpu_tick = false;
        if self.master_clock.is_multiple_of(3) {
            match self.dma_mode {
                DmaMode::None => {
                    if self.bus.cpu.rdy {
                        self.bus.cpu.tick();
                        cpu_tick = true;
                    } else {
                        // Start OAM DMA
                        self.dma_mode = DmaMode::Oam;
                        self.oam_transfer_cycles = OAM_DMA_START_CYCLES; // counts 0..511 for 256 bytes
                    }
                }
                DmaMode::Oam => {
                    // DMA transfers one byte per 2 CPU cycles
                    if self.oam_transfer_cycles < OAM_DMA_DONE_CYCLES {
                        if self.oam_transfer_cycles.is_multiple_of(2) {
                            let byte_index = self.oam_transfer_cycles / 2;

                            // Read from CPU memory
                            let hi: u16 = (self.bus.oam_dma_addr as u16) << 8;
                            let addr = hi + byte_index as u16;
                            let value = self.bus.cpu_bus_read(addr);

                            self.bus.ppu.write_to_oam_data(value);
                        }
                        self.oam_transfer_cycles += 1;
                    } else {
                        // DMA complete
                        self.dma_mode = DmaMode::None;
                        self.bus.cpu.rdy = true; // Carry on with regular CPU cycles
                    }
                }
            }

            // APU is clocked at CPU speed
            self.bus.apu.clock();
        }

        // Tick PPU
        let frame_ready = self.bus.ppu.tick();

        self.master_clock += 1;
        (cpu_tick, frame_ready)
    }

    pub fn get_frame_buffer(&self) -> &[u8; 256 * 240] {
        &self.bus.ppu.frame_buffer
    }
}
