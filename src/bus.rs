use crate::cartridge::Cartridge;
use crate::controller::joypad::Joypad;
use crate::controller::NesController;
use crate::cpu::processor::{CPU, CpuInterface};
use crate::ppu::{PPU, PpuInterface};

const CPU_RAM_SIZE: usize = 2048;
const CPU_RAM_START: u16 = 0x0000;
const CPU_RAM_END: u16 = 0x1FFF;
const CPU_MIRROR_MASK: u16 = 0b0000_0111_1111_1111;

const PPU_REGISTERS_START: u16 = 0x2000;
const PPU_REGISTERS_END: u16 = 0x3FFF;
const PPU_MIRROR_MASK: u16 = 0b0010_0000_0000_0111;
const ROM_START: u16 = 0x8000;
const ROM_END: u16 = 0xFFFF;


pub struct Bus {
    // cart: Rc<RefCell<dyn Cartridge>>,
    cart: Box<dyn Cartridge>,

    pub cpu_ram: [u8; CPU_RAM_SIZE],
    pub cycles: usize,
    pub cpu: CPU,
    pub ppu: PPU,
    pub disable_mirroring: bool,

    // Some games expect an "open-bus": When reading from invalid addresses,
    // the bus should return its last-read value
    pub last_fetched_byte: u8,

    // gameloop_callback: Box<dyn FnMut(&PPU, &mut Joypad) + 'call>,
    pub frame_complete: bool,
    controller1: Box<dyn NesController>,
}

// pub trait BusMemory {
//     type DisableMirroring;
//     fn fetch_byte(&mut self, address: u16) -> u8;
//     fn store_byte(&mut self, address: u16, value: u8);
//
//     fn fetch_u16(&mut self, address: u16) -> u16 {
//         let lo = self.fetch_byte(address) as u16;
//         let hi = self.fetch_byte(address.wrapping_add(1)) as u16;
//         hi << 8 | lo
//     }
// }

// impl Bus {
//     // type DisableMirroring = bool;
//
//     fn fetch_byte(&mut self, address: u16) -> u8 {
//         if self.disable_mirroring {
//             return *self.cpu_ram.read(address as usize);
//         }
//         let fetched_byte = match address {
//             CPU_RAM_START..=CPU_RAM_END => {
//                 let mirrored_address = address & CPU_MIRROR_MASK;
//                 *self.cpu_ram.read(mirrored_address as usize)
//             }
//             PPU_REGISTERS_START..=PPU_REGISTERS_END => {
//                 let mirrored_address = address & PPU_MIRROR_MASK;
//                 match mirrored_address {
//                     0x2002 => self.ppu.read_status(),
//                     0x2004 => self.ppu.read_oam_data(),
//                     0x2007 => self.ppu.read_data(),
//                     _ => {
//                         println!(
//                             "Attempt to read from write-only PPU register ${:04X}. Returning last_fetched_byte: {:02X}",
//                             address,
//                             self.last_fetched_byte
//                         );
//                         self.last_fetched_byte
//                     }
//                 }
//             }
//             ROM_START..=ROM_END => self.cart.borrow_mut().prg_read(address),
//
//             0x4000..=0x4015 => {
//                 // ignore APU
//                 0
//             }
//             0x4016 => {
//                 let result = self.joypad1.read();
//                 result
//             }
//             0x4017 => {
//                 // ignore joypad 2 for now
//                 0
//             }
//             0x4020..=0xFFFF => self.cart.borrow_mut().prg_read(address),
//             _ => self.last_fetched_byte,
//         };
//         self.last_fetched_byte = fetched_byte;
//         fetched_byte
//     }
//     fn store_byte(&mut self, address: u16, value: u8) {
//         if self.disable_mirroring {
//             self.cpu_ram.write(address as usize, value);
//             return;
//         }
//
//         match address {
//             CPU_RAM_START..=CPU_RAM_END => {
//                 let mirrored_address = address & CPU_MIRROR_MASK;
//                 self.cpu_ram.write(mirrored_address as usize, value);
//             }
//             PPU_REGISTERS_START..=PPU_REGISTERS_END => {
//                 let mirror_down_address = address & 0b0010_0000_0000_0111;
//                 match mirror_down_address {
//                     0x2000 => self.ppu.write_to_ctrl(value),
//                     0x2001 => self.ppu.write_to_mask(value),
//                     0x2002 => {
//                         // println!("Ignored write to $2002: {:02X}", value);
//                     }
//                     0x2003 => self.ppu.set_oam_addr(value),
//                     0x2004 => self.ppu.write_to_oam_data(value),
//                     0x2005 => self.ppu.write_to_scroll(value),
//                     0x2006 => self.ppu.set_ppu_addr(value),
//                     0x2007 => self.ppu.write_to_data(value),
//                     _ => panic!("Invalid mirrored PPU register write: ${:04X}", address),
//                 }
//             }
//             0x4000..=0x4013 | 0x4015 => {
//                 // TODO: implement APU
//             }
//             ROM_START..=ROM_END => {
//                 // Open-bus writes to ROM are ignored
//             }
//             0x4014 => {
//                 let hi: u16 = (value as u16) << 8;
//                 let mut buffer: [u8; 256] = [0; 256];
//
//                 for i in 0..256 {
//                     buffer[i] = self.fetch_byte(hi + i as u16);
//                 }
//
//                 self.ppu.write_to_oam_dma(&buffer);
//                 // TODO: NES pauses CPU for 512 cycles during DMA
//             }
//             0x4016 => {
//                 self.joypad1.write(value);
//             }
//             0x4017 => {
//                 // ignore joypad 2
//             }
//             0x4018..=0x401F => {
//                 // usually disabled
//             }
//             0x4020..=0xFFFF => {
//                 self.cart.borrow_mut().prg_write(address, value);
//             }
//             _ => {
//                 // With NROMs these are basically NOPs
//                 // Other mappers will use these when implemented
//             }
//         }
//     }
// }

impl Bus {
    pub fn new(cartridge: Box<dyn Cartridge>) -> Self {
        let mut bus = Bus {
            cart: cartridge,
            cpu_ram: [0; CPU_RAM_SIZE],
            cycles: 0,
            cpu: CPU::new(),
            ppu: PPU::new(),
            disable_mirroring: false,
            last_fetched_byte: 0,
            frame_complete: false,
            controller1: Box::new(Joypad::new()),
        };

        // Safety: This raw pointer should remain stable
        let bus_ptr = &mut bus as *mut Bus;

        // Give PPU a pointer back to the Bus (for NMI/IRQ signaling)
        bus.cpu.connect_bus(bus_ptr as *mut dyn CpuInterface);
        bus.ppu.connect_bus(bus_ptr as *mut dyn PpuInterface);

        bus
    }

    // pub fn enable_test_mode(&mut self) {
    //     self.disable_mirroring = true;
    //     self.cpu_ram.data = std::mem::take(&mut self.cart.borrow().get_prg_rom());
    //     self.cpu_ram.data.resize(1 << 16, 0u8);
    // }

    // pub fn tick(&mut self, cycles: usize) {
    //     self.cycles += cycles;
    //
    //     // let pre_nmi = self.ppu.nmi_interrupt.is_some();
    //     self.ppu.tick(cycles * 3);
    //     // let post_nmi = self.ppu.nmi_interrupt.is_some();
    //
    //     // if !pre_nmi && post_nmi {
    //     //     (self.gameloop_callback)(&self.ppu, &mut self.joypad1);
    //     // }
    // }

    pub fn get_nmi_status(&mut self) -> Option<u8> {
        self.ppu.get_nmi_status()
    }

    pub fn fetch_bytes(&mut self, address: u16, size: u8) -> &[u8] {
        // self.cpu_ram.read_n(address as usize, size as usize)
        let start = address as usize;
        let end = address.wrapping_add(size as u16) as usize;
        &self.cpu_ram[start..end]
    }

    // pub fn fetch_bytes_raw(&mut self, address: u16, size: u16) -> &[u8] {
    //     self.cpu_ram.read_n(address as usize, size as usize)
    // }

    pub fn store_bytes(&mut self, address: u16, values: &[u8]) {
        // self.cpu_ram.write_n(address as usize, values);
        let start = address as usize;
        let end = start + values.len();
        self.cpu_ram[start..end].copy_from_slice(values);
    }

    pub fn store_byte_vec(&mut self, address: u16, values: Vec<u8>) {
        // self.cpu_ram
        //     .write_n(address as usize, &values.into_boxed_slice())
        let start = address as usize;
        let end = start + values.len();
        self.cpu_ram[start..end].copy_from_slice(&values);
    }

    pub fn poll_frame_complete(&mut self) -> bool {
        let result = self.frame_complete;
        if self.frame_complete {
            self.frame_complete = false;
        }
        result
    }
}


impl CpuInterface for Bus {
    fn read(&mut self, addr: u16) -> u8 {
        println!("\tCpuInterface::read()");
        0
    }

    fn write(&mut self, addr: u16, value: u8) {
        println!("\tCpuInterface::write()");
    }

    fn signal_nmi(&mut self) {
        todo!()
    }
}

impl PpuInterface for Bus {
    fn read(&mut self, addr: u16) -> u8 {
        todo!()
    }

    fn write(&mut self, addr: u16, value: u8) {
        todo!()
    }
}



