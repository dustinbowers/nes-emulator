use cpu::CpuBusInterface;
pub mod apu;
pub mod bus;
pub mod cartridge;
pub mod controller;
pub mod cpu;
pub mod ppu;
pub mod tracer;

pub mod dmc_dma;
mod oam_dma;
#[cfg(feature = "testing-utils")]
pub mod test_utils;

use crate::nes::apu::ApuBusInterface;
use crate::nes::dmc_dma::DmcDma;
use crate::nes::oam_dma::{OamDma, OamDmaOp};
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

pub enum BusOwner {
    Cpu,
    Dmc,
    Oam,
}

pub struct NES {
    pub run_state: RunState,
    pub bus: &'static mut NesBus,
    master_clock: u64,
    cpu_cycle_parity: bool,

    dmc_dma: DmcDma,
    oam_dma: OamDma,
    oam_byte: u8,

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

            cpu_cycle_parity: false,
            dmc_dma: DmcDma::new(),
            oam_dma: OamDma::new(),
            oam_byte: 0,

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
        let mut cpu_ticked = false;
        if self.master_clock.is_multiple_of(3) {
            // Handle OAM request
            if let Some(page) = self.bus.oam_dma_request.take() {
                self.oam_dma.start(page, self.cpu_cycle_parity);
            }
            if self.bus.apu.dmc.wants_bus() {
                let addr = self.bus.apu.dmc.dma_addr();
                self.dmc_dma.request(addr);
            }
            let dmc_active = self.dmc_dma.active();
            let oam_active = self.oam_dma.active();
            let dma_active = dmc_active || oam_active;

            if !dmc_active && self.dmc_dma.pending() {
                self.dmc_dma.begin();
            }

            let dma_active_now = self.dmc_dma.active() || oam_active;
            let rdy_line = !dma_active_now;

            // Tick CPU
            let (stalled, _, _) = self.bus.cpu.tick(rdy_line);
            cpu_ticked = !stalled;

            if stalled {
                if self.dmc_dma.active() {
                    if let Some(addr) = self.dmc_dma.step() {
                        let byte = self.bus.apu_bus_read(addr);
                        self.bus.apu.dmc.supply_dma_byte(byte);
                    }
                } else if oam_active {
                    let oam_op = self.oam_dma.step();
                    match oam_op {
                        OamDmaOp::Dummy => {}
                        OamDmaOp::Read(addr) => {
                            self.oam_byte = self.bus.cpu_bus_read(addr);
                        }
                        OamDmaOp::Write => {
                            self.bus.ppu.write_to_oam_data(self.oam_byte);
                        }
                    }
                }
            }

            // APU is clocked at CPU speed
            self.bus.apu.clock();

            self.cpu_cycle_parity = !self.cpu_cycle_parity;
        }

        // Tick PPU
        let frame_ready = self.bus.ppu.tick();

        self.master_clock = (self.master_clock + 1) % 3_000_000;
        (cpu_ticked, frame_ready)
    }

    pub fn get_frame_buffer(&self) -> &[u8; 256 * 240] {
        &self.bus.ppu.frame_buffer
    }
}
