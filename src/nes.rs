use crate::bus::nes_bus::NesBus;
use crate::cartridge::Cartridge;
use crate::cpu::processor::CpuBusInterface;

const PPU_WARMUP_CYCLES: usize = 89343;

const OAM_DMA_START_CYCLES: usize = 512;
const OAM_DMA_DONE_CYCLES: usize = 0;
enum DmaMode {
    Oam,
    None,
}
pub struct NES {
    pub bus: &'static mut NesBus,
    pub ppu_warmed_up: bool,
    pub ppu_cycles: usize,
    pub cpu_cycle_debt: f32,
    dma_mode: DmaMode,

    pub oam_dma_skip_cycles: usize,

    audio_time_per_system_sample: f32,
    audio_time_per_nes_clock: f32,
}

impl NES {
    pub fn new(cartridge: Box<dyn Cartridge>) -> Self {
        let bus = NesBus::new(cartridge);
        Self {
            bus,
            ppu_cycles: 0,
            cpu_cycle_debt: 0.0,
            ppu_warmed_up: false,
            dma_mode: DmaMode::None,
            oam_dma_skip_cycles: 0,
            audio_time_per_system_sample: 0.0,
            audio_time_per_nes_clock: 0.0,
        }
    }

    // Tick once at PPU frequency
    pub fn tick(&mut self) -> bool {
        self.ppu_cycles += 1;

        // PPU warmup (89343 PPU cycles = 29781 CPU cycles × 3)
        if !self.ppu_warmed_up {
            if self.ppu_cycles > PPU_WARMUP_CYCLES {
                println!("=== PPU WARMED UP at {} PPU cycles", self.ppu_cycles);
                self.ppu_warmed_up = true;
            }
            return false;
        }

        // Tick PPU
        let frame_ready = self.bus.ppu.tick();

        // CPU runs at 1/3 PPU speed
        self.cpu_cycle_debt += 1.0 / 3.0;

        if self.cpu_cycle_debt >= 1.0 {
            self.cpu_cycle_debt -= 1.0;

            match self.dma_mode {
                DmaMode::None => {
                    if !self.bus.cpu.rdy {
                        self.dma_mode = DmaMode::Oam;
                        self.oam_dma_skip_cycles = OAM_DMA_START_CYCLES;
                    } else {
                        self.bus.cpu.tick();
                    }
                }
                DmaMode::Oam => {
                    if self.oam_dma_skip_cycles == OAM_DMA_START_CYCLES {
                        let hi: u16 = (self.bus.oam_dma_addr as u16) << 8;
                        let mut buffer: [u8; 256] = [0; 256];
                        for i in 0..256 {
                            buffer[i] = self.bus.cpu_bus_read(hi + i as u16);
                        }
                        self.bus.ppu.write_to_oam_dma(&buffer);
                    }

                    if self.oam_dma_skip_cycles == OAM_DMA_DONE_CYCLES {
                        self.dma_mode = DmaMode::None;
                        self.bus.cpu.rdy = true;
                    } else {
                        self.oam_dma_skip_cycles -= 1;
                    }
                }
            }
            
            // APU runs at CPU speed (every CPU cycle)
            let cpu_cycle_count = self.ppu_cycles / 3;
            self.bus.apu.clock(cpu_cycle_count);
        }
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
