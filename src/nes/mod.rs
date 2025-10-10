pub mod apu;
pub mod bus;
pub mod cartridge;
pub mod cpu;
pub mod ppu;
pub mod tracer;

pub mod controller;

use bus::nes_bus::NesBus;
use cartridge::Cartridge;
use cpu::processor::CpuBusInterface;
use crate::{trace, trace_obj};

const OAM_DMA_START_CYCLES: usize = 0;
const OAM_DMA_DONE_CYCLES: usize = 512;
const PPU_WARMUP_CYCLES: usize = 89343;

enum DmaMode {
    Oam,
    None,
}
pub struct NES {
    pub bus: &'static mut NesBus,
    pub ppu_warmed_up: bool,
    dma_mode: DmaMode,

    pub oam_transfer_cycles: usize,

    audio_time_per_system_sample: f32,
    audio_time_per_nes_clock: f32,
}

impl NES {
    pub fn new(cartridge: Box<dyn Cartridge>) -> Self {
        let bus = NesBus::new(cartridge);
        Self {
            bus,
            ppu_warmed_up: true,
            dma_mode: DmaMode::None,
            oam_transfer_cycles: 0,
            audio_time_per_system_sample: 0.0,
            audio_time_per_nes_clock: 0.0,
        }
    }

    // Tick once at PPU frequency
    pub fn tick(&mut self) -> bool {
        // Tick PPU
        let frame_ready = self.bus.ppu.tick();
        // trace_obj!(self.bus.ppu);
        
        // Runs at PPU speed
        self.bus.tick(); 
        
        // CPU runs at 1/3 PPU speed
        if self.bus.ppu.global_ppu_ticks.is_multiple_of(3) {
            match self.dma_mode {
                DmaMode::None => {
                    if !self.bus.cpu.rdy {
                        // Start OAM DMA
                        // println!("DMA START");
                        trace!("PPU DMA START");
                        self.dma_mode = DmaMode::Oam;
                        self.oam_transfer_cycles = OAM_DMA_START_CYCLES; // counts 0..511 for 256 bytes
                        self.bus.cpu.rdy = false;
                    } else {
                        self.bus.cpu.tick();
                        trace_obj!(self.bus.cpu);
                    }
                }
                DmaMode::Oam => {
                    // DMA transfers one byte per 2 CPU cycles
                    if self.oam_transfer_cycles < OAM_DMA_DONE_CYCLES {
                        let byte_index = self.oam_transfer_cycles / 2;
                        if self.oam_transfer_cycles.is_multiple_of(2) {
                            // Read from CPU memory
                            let hi: u16 = (self.bus.oam_dma_addr as u16) << 8;
                            let addr = hi + byte_index as u16;
                            let value = self.bus.cpu_bus_read(addr);
                            // trace!("[OAM TRANSFER] oam_transfer_cycles={} addr={} cpu_byte={:02X}",
                            //     self.oam_transfer_cycles, addr, value);
                            self.bus.ppu.write_to_oam_data(value);
                        }
                        self.oam_transfer_cycles += 1;
                    } else {
                        // DMA complete
                        trace!("PPU DMA COMPLETE");
                        self.dma_mode = DmaMode::None;
                        self.bus.cpu.rdy = true;
                    }
                }
            }

            // Runs at CPU speed
            self.bus.apu.clock(self.bus.cpu.cycles);
        }
       
        // trace!("NES: cpu_cycles={} ppu_cycles={}", self.bus.cpu.cycles, self.bus.ppu.global_ppu_ticks);
        frame_ready
    }

    pub fn set_sample_frequency(&mut self, sample_rate: u32) {
        self.audio_time_per_system_sample = 1.0 / (sample_rate as f32);
        self.audio_time_per_nes_clock = 1.0 / 1789773.0; // CPU clock frequency (NTSC)

        println!(
            "audio_time_per_system_sample: {}",
            self.audio_time_per_system_sample
        );
        println!(
            "audio_time_per_nes_clock: {}",
            self.audio_time_per_nes_clock
        );
    }

    pub fn get_frame_buffer(&self) -> &[u8; 256 * 240] {
        &self.bus.ppu.frame_buffer
    }
}
