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
    pub dma_mode: DmaMode,

    pub oam_dma_skip_cycles: usize,
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
        }
    }

    pub fn tick(&mut self) -> bool {
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

    pub fn get_frame_buffer(&self) -> &[u8; 256 * 240] {
        return &self.bus.ppu.frame_buffer;
    }

    #[deprecated]
    pub fn clear_frame(&mut self) {
        // Note: this takes way longer that I was expecting...
        // TODO: Would double-buffering be faster than blitting a bunch of zeros?
        self.bus.ppu.frame_buffer.fill(0);
    }
}
