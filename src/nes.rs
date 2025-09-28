use crate::bus::nes_bus::NesBus;
use crate::cartridge::Cartridge;
use crate::cpu::processor::CpuBusInterface;

const PPU_WARMUP_CYCLES: usize = 29781;

const OAM_DMA_START_CYCLES: usize = 512;
const OAM_DMA_DONE_CYCLES: usize = 0;
enum DmaMode {
    OAM,
    None,
}
pub struct NES {
    pub bus: &'static mut NesBus,
    pub ppu_warmed_up: bool,
    pub cpu_cycles: usize,
    dma_mode: DmaMode,

    pub oam_dma_skip_cycles: usize,

    audio_time_per_system_sample: f32,
    audio_time_per_nes_clock: f32,
    audio_time: f32,
    audio_sample_ready: bool,
}

impl NES {
    pub fn new(cartridge: Box<dyn Cartridge>) -> Self {
        let bus = NesBus::new(cartridge);
        Self {
            bus,
            cpu_cycles: 0,
            ppu_warmed_up: false,
            dma_mode: DmaMode::None,
            oam_dma_skip_cycles: 0,
            audio_time_per_system_sample: 0.0,
            audio_time_per_nes_clock: 0.0,
            audio_time: 0.0,
            audio_sample_ready: false,
        }
    }

    pub fn clock(&mut self) -> bool {
        match self.dma_mode {
            DmaMode::None => {
                // Check if we're ready to switch to OAM DMA
                if self.bus.cpu.rdy == false {
                    // This is effectively a 1-cycle for the "halt"
                    self.dma_mode = DmaMode::OAM;
                    self.oam_dma_skip_cycles = OAM_DMA_START_CYCLES;
                    return false;
                }

                // CPU
                self.bus.cpu.tick();
                self.cpu_cycles += 1;
                if self.cpu_cycles > 1_000_000 {
                    self.cpu_cycles -= 1_000_000;
                }

                let mut frame_ready = false;

                if self.ppu_warmed_up {
                    // PPU ticks 3 times per CPU cycle
                    for _ in 0..3 {
                        // self.bus.ppu.tick();
                        if self.bus.ppu.tick() {
                            frame_ready = true;
                        }
                    }
                } else if self.cpu_cycles > PPU_WARMUP_CYCLES {
                    println!("=== PPU WARMED UP at {} cpu_cycles cycles", self.cpu_cycles);
                    self.ppu_warmed_up = true;
                }

                // master clock is CPU
                self.bus.apu.clock(self.cpu_cycles);

                self.audio_sample_ready = false;
                self.audio_time += self.audio_time_per_nes_clock;
                if self.audio_time >= self.audio_time_per_system_sample {
                    self.audio_time -= self.audio_time_per_system_sample;
                    self.audio_sample_ready = true;
                }

                self.audio_sample_ready
                // frame_ready
            }
            DmaMode::OAM => {
                if self.oam_dma_skip_cycles == OAM_DMA_START_CYCLES {
                    // TODO: technically this should happen sequentially over 512 cycles
                    //       but that timing shouldn't be that important...

                    // Just copy everything directly for now
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
                    return false;
                }

                self.oam_dma_skip_cycles -= 1;
                false
            }
        }
    }

    pub fn set_sample_frequency(&mut self, sample_rate: u32) {
        self.audio_time_per_system_sample = 1.0 / (sample_rate as f32);
        // self.audio_time_per_nes_clock = 1.0 / 5369318.0; // PPU clock frequency (NTSC)
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
        return &self.bus.ppu.frame_buffer;
    }
}
