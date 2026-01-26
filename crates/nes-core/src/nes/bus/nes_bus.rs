use crate::nes::apu::{APU, ApuBusInterface};
use crate::nes::bus::consts::*;
use crate::nes::cartridge::Cartridge;
use crate::nes::cartridge::rom::Mirroring;
use crate::nes::controller::NesController;
use crate::nes::controller::joypad::Joypad;
use crate::nes::cpu::{CPU, CpuBusInterface};
use crate::nes::ppu::{PPU, PpuBusInterface};

use crate::trace;

pub struct NesBus {
    cart: Option<Box<dyn Cartridge>>,

    pub cpu_ram: [u8; CPU_RAM_SIZE],
    pub cpu: CPU,
    pub ppu: PPU,
    pub apu: APU,

    pub nmi_scheduled: Option<u8>,

    pub oam_dma_addr: u8,

    // Some games expect an "open-bus":
    // i.e. invalid reads return last-read byte
    pub last_cpu_read: u8,

    pub joypads: [Joypad; 2],
    // pub controller1: Box<Joypad>,
    // pub controller2: Box<Joypad>,
}

impl NesBus {
    pub fn new() -> &'static mut NesBus {
        let mut bus = Box::new(NesBus {
            cart: None,
            cpu_ram: [0; CPU_RAM_SIZE],
            cpu: CPU::new(),
            ppu: PPU::new(),
            apu: APU::new(),

            nmi_scheduled: None,
            oam_dma_addr: 0,

            last_cpu_read: 0,
            joypads: [Joypad::new(), Joypad::new()],
            // controller1: Box::new(Joypad::new()),
            // controller2: Box::new(Joypad::new()),
        });

        // Safety: This raw pointer should remain stable
        let bus_ptr: *mut NesBus = &mut *bus;

        // Give CPU/PPU a pointer back to the Bus
        bus.cpu.connect_bus(bus_ptr as *mut dyn CpuBusInterface);
        bus.ppu.connect_bus(bus_ptr as *mut dyn PpuBusInterface);

        Box::leak(bus)
    }

    pub fn reset_components(&mut self) {
        self.cpu_ram = [0; CPU_RAM_SIZE];
        self.nmi_scheduled = None;
        self.oam_dma_addr = 0;
        self.last_cpu_read = 0;
        self.joypads = [Joypad::new(), Joypad::new()];
        // *self.controller1 = Joypad::new();

        self.cpu.reset();
        self.ppu.reset();
        self.apu.reset();
    }

    #[allow(dead_code)]
    pub fn new_with_cartridge(cart: Box<dyn Cartridge>) -> &'static mut NesBus {
        let bus = NesBus::new();
        bus.insert_cartridge(cart);
        bus
    }

    pub fn insert_cartridge(&mut self, cart: Box<dyn Cartridge>) {
        self.cart = Some(cart);
        self.reset_components();
    }
}

impl CpuBusInterface for NesBus {
    fn cpu_bus_read(&mut self, addr: u16) -> u8 {
        let value = match addr {
            CPU_RAM_START..=CPU_RAM_END => {
                // RAM mirrored every 0x0800
                let mirrored = addr & 0x07FF;
                self.cpu_ram[mirrored as usize]
            }
            PPU_REGISTERS_START..=PPU_REGISTERS_END => {
                let mirrored_addr = 0x2000 + (addr & 7);
                if mirrored_addr == 0x2002 && self.ppu.status_register.vblank_active() {
                    trace!(
                        "[CPU READ $2002] PC=${:04X} PPU={}{} dot={} vblank={}",
                        self.cpu.program_counter,
                        self.ppu.scanline,
                        if self.ppu.scanline < 10 { " " } else { "" },
                        self.ppu.cycles + 1,
                        self.ppu.status_register.vblank_active()
                    );
                }

                // PPU Registers mirrored every 8 bytes
                self.ppu.read_register(addr)
            }
            // 0x4000..=0x4013 => {
            //     // panic!("reading apu register: {:04X}", addr);
            //
            // }
            0x4015 => self.apu.read(addr),
            0x4014 => {
                // Open bus
                self.last_cpu_read
            }
            0x4016 => self.joypads[0].read(),
            0x4017 => self.joypads[1].read(),
            0x4018..=0x401F => {
                // Open bus
                self.last_cpu_read
            }
            CART_START..=CART_END => match &mut self.cart {
                Some(cart) => cart.cpu_read(addr),
                None => 0,
            },
            _ => self.last_cpu_read,
        };
        self.last_cpu_read = value;
        value
    }

    fn cpu_bus_write(&mut self, addr: u16, value: u8) {
        match addr {
            CPU_RAM_START..=CPU_RAM_END => {
                let mirrored = addr & 0x07FF;
                self.cpu_ram[mirrored as usize] = value;
            }
            PPU_REGISTERS_START..=PPU_REGISTERS_END => {
                self.ppu.write_register(addr, value);
            }
            0x4014 => {
                self.cpu.halt_scheduled = true;
                self.oam_dma_addr = value;
            }
            0x4016 => {
                // Used to reset strobing via bit 0
                self.joypads[0].write(value);
                self.joypads[1].write(value);
            }
            0x4000..=0x4013 | 0x4015 | 0x4017 => {
                // APU
                self.apu.write(addr, value);
            }
            0x4018..=0x401F => { /* Open bus */ }
            CART_START..=CART_END => {
                if let Some(cart) = &mut self.cart {
                    cart.cpu_write(addr, value);
                }
            }
            _ => {}
        }
    }
}

impl PpuBusInterface for NesBus {
    fn chr_read(&mut self, addr: u16) -> u8 {
        match &mut self.cart {
            Some(cart) => cart.ppu_read(addr),
            _ => 0,
        }
    }
    fn chr_write(&mut self, addr: u16, value: u8) {
        if let Some(cart) = &mut self.cart {
            cart.ppu_write(addr, value);
        }
    }
    fn mirroring(&mut self) -> Mirroring {
        match &self.cart {
            Some(cart) => cart.mirroring(),
            None => Mirroring::Horizontal,
        }
    }
    fn nmi(&mut self) {
        trace!("CPU Triggering NMI!");
        self.cpu.trigger_nmi();
    }
}

impl ApuBusInterface for NesBus {
    fn apu_bus_read(&mut self, addr: u16) -> u8 {
        println!("ApuBusInterface::read({:?})", addr);
        0
    }
    fn irq(&mut self) {
        println!("ApuBusInterface::irq()");
        // TODO: Set IRQ request
    }
}

#[cfg(feature = "tracing")]
use crate::nes::tracer::traceable::Traceable;
#[cfg(feature = "tracing")]
impl Traceable for NesBus {
    fn trace_name(&self) -> &'static str {
        "BUS"
    }

    fn trace_state(&self) -> Option<String> {
        let cpu_trace = self.cpu.trace().unwrap_or("---".to_string());
        let ppu_trace = self.ppu.trace().unwrap_or("---".to_string());
        Some(format!("{} | {} ", cpu_trace, ppu_trace))
    }
}
