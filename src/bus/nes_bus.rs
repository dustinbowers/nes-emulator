use crate::cartridge::Cartridge;
use crate::controller::joypad::Joypad;
use crate::controller::NesController;
use crate::cpu::processor::{CpuBusInterface, CPU};
use crate::ppu::{PpuBusInterface, PPU};
use crate::rom::Mirroring;

const CPU_RAM_SIZE: usize = 2048;
const CPU_RAM_START: u16 = 0x0000;
const CPU_RAM_END: u16 = 0x1FFF;

const PPU_REGISTERS_START: u16 = 0x2000;
const PPU_REGISTERS_END: u16 = 0x3FFF;
const CART_START: u16 = 0x4200;
const CART_END: u16 = 0xFFFF;

pub struct NesBus {
    cart: Box<dyn Cartridge>,

    pub cpu_ram: [u8; CPU_RAM_SIZE],
    pub cycles: usize,
    pub cpu: CPU,
    pub ppu: PPU,

    pub oam_dma_addr: u8,

    // Some games expect an "open-bus":
    // i.e. invalid reads return last-read byte
    pub last_fetched_byte: u8,

    pub controller1: Box<Joypad>,
    // TODO: controller2: Box<dyn NexController>,
}

impl NesBus {
    pub fn new(cartridge: Box<dyn Cartridge>) -> &'static mut NesBus {
        let mut bus = Box::new(NesBus {
            cart: cartridge,
            cpu_ram: [0; CPU_RAM_SIZE],
            cycles: 0,
            cpu: CPU::new(),
            ppu: PPU::new(),

            oam_dma_addr: 0,

            last_fetched_byte: 0,
            controller1: Box::new(Joypad::new()),
        });

        // Safety: This raw pointer should remain stable
        let bus_ptr: *mut NesBus = &mut *bus;

        // Give PPU a pointer back to the Bus (for NMI/IRQ signaling)
        bus.cpu.connect_bus(bus_ptr as *mut dyn CpuBusInterface);
        bus.ppu.connect_bus(bus_ptr as *mut dyn PpuBusInterface);

        Box::leak(bus)
    }
}

impl CpuBusInterface for NesBus {
    fn cpu_bus_read(&mut self, addr: u16) -> u8 {
        match addr {
            CPU_RAM_START..=CPU_RAM_END => {
                // RAM mirrored every 0x0800
                let mirrored = addr & 0x07FF;
                self.cpu_ram[mirrored as usize]
            }
            PPU_REGISTERS_START..=PPU_REGISTERS_END => {
                // PPU Registers mirrored every 8 bytes
                let reg = 0x2000 + (addr & 0x0007);
                self.ppu.read_register(reg)
            }
            0x4000..=0x4013 | 0x4015 => {
                // TODO: APU
                0
            }
            0x4014 => {
                // Open bus
                unimplemented!("Invalid CPU address read: ${:04X}", addr);
            }
            0x4016 => self.controller1.read(),
            0x4017 => {
                /* self.controller2.read() */
                0
            }
            0x4018..=0x401F => {
                // Open bus
                unimplemented!("Invalid CPU address read: ${:04X}", addr);
            }
            CART_START..=CART_END => {
                let byte = self.cart.prg_read(addr);
                byte
            }
            _ => 0,
        }
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
            0x4017 => {
                // TODO: later
                /* self.controller2.write(value) */
            }
            0x4018..=0x401F => { /* Open bus */ }
            CART_START..=CART_END => self.cart.prg_write(addr, value),
            _ => {
                // println!("Unhandled CPU write at {:04X}", addr);
            }
        }
    }
}

impl PpuBusInterface for NesBus {
    fn chr_read(&mut self, addr: u16) -> u8 {
        self.cart.chr_read(addr)
    }
    fn chr_write(&mut self, addr: u16, value: u8) {
        self.cart.chr_write(addr, value);
    }
    fn mirroring(&mut self) -> Mirroring {
        self.cart.mirroring()
    }
    fn nmi(&mut self) {
        self.cpu.trigger_nmi();
    }
}
