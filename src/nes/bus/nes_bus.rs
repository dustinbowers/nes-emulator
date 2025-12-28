use crate::nes::apu::{APU, ApuBusInterface};
use crate::nes::cartridge::Cartridge;
use crate::nes::cartridge::rom::Mirroring;
use crate::nes::controller::NesController;
use crate::nes::controller::joypad::Joypad;
use crate::nes::cpu::processor::{CPU, CpuBusInterface};
use crate::nes::ppu::registers::status_register::StatusRegister;
use crate::nes::ppu::{PPU, PpuBusInterface};
use crate::nes::tracer::traceable::Traceable;
use crate::trace;
use std::pin::Pin;

const CPU_RAM_SIZE: usize = 2048;
const CPU_RAM_START: u16 = 0x0000;
const CPU_RAM_END: u16 = 0x1FFF;

const PPU_REGISTERS_START: u16 = 0x2000;
const PPU_REGISTERS_END: u16 = 0x3FFF;
const CART_START: u16 = 0x4200;
const CART_END: u16 = 0xFFFF;

pub struct NesBus {
    cart: Option<Box<dyn Cartridge>>,

    pub cpu_ram: [u8; CPU_RAM_SIZE],
    // pub cycles: usize,
    pub cpu: CPU,
    pub ppu: PPU,
    pub apu: APU,

    pub nmi_scheduled: Option<u8>,

    pub oam_dma_addr: u8,

    // Some games expect an "open-bus":
    // i.e. invalid reads return last-read byte
    pub last_cpu_read: u8,

    pub controller1: Box<Joypad>,
    // TODO: controller2: Box<dyn NexController>,
}

impl NesBus {
    pub fn new() -> &'static mut NesBus {
        let mut bus = Box::new(NesBus {
            cart: None,
            cpu_ram: [0; CPU_RAM_SIZE],
            // cycles: 0,
            cpu: CPU::new(),
            ppu: PPU::new(),
            apu: APU::new(),

            nmi_scheduled: None,
            oam_dma_addr: 0,

            last_cpu_read: 0,
            controller1: Box::new(Joypad::new()),
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
        self.controller1 = Box::new(Joypad::new());

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
        println!("NesBus::insert_cartridge()");
        self.cart = Some(cart);
        self.reset_components();
    }

    pub fn tick(&mut self) {
        if let Some(ct) = self.nmi_scheduled {
            if ct == 0 {
                self.cpu.trigger_nmi();
                // println!("triggering NMI!");
                trace!("PPU triggering NMI!");
                self.nmi_scheduled = None;
            } else {
                // println!("decrementing NMI: {}", ct);
                self.nmi_scheduled = Some(ct - 1)
            }
        }
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
                // PPU Registers mirrored every 8 bytes
                // let reg = 0x2000 + (addr & 0x0007);
                if addr == 0x2002 {
                    trace!(
                        "{}",
                        format!(
                            "[CPU READ $2002] CPU_PC=${:04X} PPU global_cycles={} SL={} DOT={}, (cpu_view_vblank={:?})",
                            self.cpu.program_counter,
                            self.ppu.global_ppu_ticks,
                            self.ppu.scanline,
                            self.ppu.cycles,
                            self.ppu
                                .status_register
                                .contains(StatusRegister::VBLANK_STARTED)
                        )
                    );
                }
                let result = self.ppu.read_register(addr);
                result
            }
            // 0x4000..=0x4013 => {
            //     // panic!("reading apu register: {:04X}", addr);
            //
            // }
            0x4015 => self.apu.read(addr),
            0x4014 => {
                // Open bus
                // unimplemented!("Invalid CPU address read: ${:04X}", addr);
                self.last_cpu_read
            }
            0x4016 => self.controller1.read(),
            0x4017 => {
                /* self.controller2.read() */
                0
            }
            0x4018..=0x401F => {
                // Open bus
                // unimplemented!("Invalid CPU address read: ${:04X}", addr);
                self.last_cpu_read
            }
            CART_START..=CART_END => {
                // if let Some(cart) = self.cart {
                //     let byte = self.cart.prg_read(addr);
                //     byte
                // }
                match &mut self.cart {
                    Some(cart) => cart.prg_read(addr),
                    None => 0,
                }
            }
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
                self.controller1.write(value);
            }
            0x4000..=0x4013 | 0x4015 | 0x4017 => {
                // APU
                self.apu.write(addr, value);
            }

            0x4018..=0x401F => { /* Open bus */ }
            CART_START..=CART_END => {
                if let Some(cart) = &mut self.cart {
                    cart.prg_write(addr, value);
                }
            }
            _ => {
                // println!("Unhandled CPU write at {:04X}", addr);
            }
        }
    }
}

impl PpuBusInterface for NesBus {
    fn chr_read(&mut self, addr: u16) -> u8 {
        match &mut self.cart {
            Some(cart) => cart.chr_read(addr),
            _ => 0,
        }
    }
    fn chr_write(&mut self, addr: u16, value: u8) {
        if let Some(cart) = &mut self.cart {
            cart.chr_write(addr, value);
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
        // self.nmi_scheduled = Some(0);// * 3);
        // println!("scheduling NMI! {:?}", self.nmi_scheduled);
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

impl Traceable for &mut NesBus {
    fn trace_name(&self) -> &'static str {
        "BUS"
    }

    fn trace_state(&self) -> Option<String> {
        let sl = self.ppu.scanline;
        let dot = self.ppu.cycles;
        if ((241..=241).contains(&sl) || (261..=261).contains(&sl)) && (0..=10).contains(&dot) {
            let cpu_trace = self.cpu.trace().unwrap_or("---".to_string());
            let ppu_trace = self.ppu.trace().unwrap_or("---".to_string());
            let status_register_trace = self
                .ppu
                .status_register
                .trace()
                .unwrap_or("---".to_string());

            // if cpu_trace != "---" {
            if self.ppu.global_ppu_ticks.is_multiple_of(3) {
                Some(format!(
                    "{{ {} | {} | {} }}",
                    cpu_trace, ppu_trace, status_register_trace
                ))
            } else {
                None
            }
            // } else {
            //     None
            // }
        } else {
            None
        }
    }
}
