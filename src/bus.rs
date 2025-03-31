use crate::memory::heap_memory::HeapMemory;
use crate::memory::memory_trait::MemoryTrait;
use crate::ppu::PPU;
use crate::rom::Rom;

const CPU_RAM_SIZE: usize = 2048;
const CPU_RAM_START: u16 = 0x0000;
const CPU_RAM_END: u16 = 0x1FFF;
const CPU_MIRROR_MASK: u16 = 0b0000_0111_1111_1111;

const PPU_REGISTERS_START: u16 = 0x2008;
const PPU_REGISTERS_END: u16 = 0x3FFF;
const PPU_MIRROR_MASK: u16 = 0b0010_0000_0000_0111;
const ROM_START: u16 = 0x8000;
const ROM_END: u16 = 0xFFFF;

pub struct Bus {
    pub cpu_ram: HeapMemory<u8>,
    pub cycles: usize,
    prg_rom: Vec<u8>,
    pub ppu: PPU,
    pub disable_mirroring: bool,
    pub ready_to_render: bool,
}

pub trait BusMemory {
    type DisableMirroring;
    fn fetch_byte(&mut self, address: u16) -> u8;
    fn store_byte(&mut self, address: u16, value: u8);

    fn fetch_u16(&mut self, address: u16) -> u16 {
        let lo = self.fetch_byte(address) as u16;
        let hi = self.fetch_byte(address.wrapping_add(1)) as u16;
        hi << 8 | lo
    }

    fn store_u16(&mut self, address: u16, value: u16) {
        self.store_byte(address, (value >> 8) as u8);
        self.store_byte(address.wrapping_add(1), value as u8);
    }
}

impl BusMemory for Bus {
    type DisableMirroring = bool;

    fn fetch_byte(&mut self, address: u16) -> u8 {
        if self.disable_mirroring {
            return *self.cpu_ram.read(address as usize);
        }
        match address {
            CPU_RAM_START..=CPU_RAM_END => {
                let mirrored_address = address & CPU_MIRROR_MASK;
                *self.cpu_ram.read(mirrored_address as usize)
            }
            0x2000 | 0x2001 | 0x2003 | 0x2005 | 0x2006 | 0x4014 => {
                panic!("Attempt to read from write-only PPU address {:x}", address);
            }
            0x2002 => self.ppu.read_status(),
            0x2004 => self.ppu.read_oam_data(),
            0x2007 => self.ppu.read_data(),

            0x4000..=0x4015 => {
                // ignore APU
                0
            }
            0x4016 => {
                // TODO: implement joypad 1
                0
            }
            0x4017 => {
                // ignore joypad 2
                0
            }

            PPU_REGISTERS_START..=PPU_REGISTERS_END => {
                let mirrored_address = address & PPU_MIRROR_MASK;
                self.fetch_byte(mirrored_address)
            }
            ROM_START..=ROM_END => self.read_prg_rom(address),
            _ => {
                println!("Invalid fetch from ${:04X}", address);
                0
            }
        }
    }
    fn store_byte(&mut self, address: u16, value: u8) {
        // if value != 0 {
        //     println!("store_byte ${:04X} = {:02X}", address, value);
        // }
        if self.disable_mirroring {
            self.cpu_ram.write(address as usize, value);
            return;
        }
        match address {
            CPU_RAM_START..=CPU_RAM_END => {
                let mirrored_address = address & CPU_MIRROR_MASK;
                self.cpu_ram.write(mirrored_address as usize, value);
            }
            0x2000 => {
                // println!("write to ctrl - {:04X}", value);
                self.ppu.write_to_ctrl(value);
            }
            0x2001 => {
                self.ppu.write_to_mask(value);
            }
            0x2002 => panic!("attempt to write to PPU status register"),
            0x2003 => {
                self.ppu.set_oam_addr(value);
            }
            0x2004 => {
                self.ppu.write_to_oam_data(value);
            }
            0x2005 => {
                self.ppu.write_to_scroll(value);
            }
            0x2006 => {
                self.ppu.set_ppu_addr(value);
            }
            0x2007 => {
                self.ppu.write_to_data(value);
            }
            PPU_REGISTERS_START..=PPU_REGISTERS_END => {
                let mirror_down_addr = address & 0b0010_0000_0000_0111;
                self.store_byte(mirror_down_addr, value);
            }
            0x4000..=0x4013 | 0x4015 => {
                // TODO: implement APU
            }
            ROM_START..=ROM_END => {
                panic!("{}", format!("Attempted write to ROM! (${:04X})", address))
            }

            0x4014 => {
                let mut buffer: [u8; 256] = [0; 256];
                let hi: u16 = (value as u16) << 8;
                for i in 0..256u16 {
                    buffer[i as usize] = self.fetch_byte(hi + i);
                }

                self.ppu.write_to_oam_dma(&buffer);
                // println!("writing to OAM data: {:?}", &buffer);
            }
            0x4016 => {
                // TODO: implement joypad 1
            }
            0x4017 => {
                // ignore joypad 2
            }
            _ => {
                panic!("Invalid write to ${:04X}", address);
            }
        }
    }
}

impl Bus {
    pub fn new(rom: Rom, ) -> Self {
        let prg_rom = rom.prg_rom.clone();
        let ppu = PPU::new(rom.chr_rom, rom.screen_mirroring);

        Self {
            cpu_ram: HeapMemory::new(CPU_RAM_SIZE, 0u8),
            cycles: 0,
            prg_rom,
            ppu,
            disable_mirroring: false,
            ready_to_render: false,
        }
    }

    pub fn enable_test_mode(&mut self) {
        self.disable_mirroring = true;
        self.cpu_ram.data = std::mem::take(&mut self.prg_rom.clone());
        self.cpu_ram.data.resize(1 << 16, 0u8);
    }

    pub fn tick(&mut self, cycles: usize) {
        self.cycles += cycles;

        let pre_nmi = self.ppu.get_nmi_status();
        self.ppu.tick(cycles * 3);
        let post_nmi = self.ppu.get_nmi_status();
        if !pre_nmi && post_nmi {
            self.ready_to_render = true
        }
    }

    pub fn get_nmi_status(&mut self) -> bool {
        self.ppu.get_nmi_status()
    }

    pub fn fetch_bytes(&mut self, address: u16, size: u8) -> &[u8] {
        self.cpu_ram.read_n(address as usize, size as usize)
    }

    pub fn fetch_bytes_raw(&mut self, address: u16, size: u16) -> &[u8] {
        self.cpu_ram.read_n(address as usize, size as usize)
    }

    pub fn store_bytes(&mut self, address: u16, values: &[u8]) {
        self.cpu_ram.write_n(address as usize, values);
    }

    pub fn store_byte_vec(&mut self, address: u16, values: Vec<u8>) {
        self.cpu_ram
            .write_n(address as usize, &values.into_boxed_slice())
    }

    fn read_prg_rom(&self, addr: u16) -> u8 {
        let addr = addr - 0x8000;

        // Calculate the effective address with mirroring if needed
        let effective_addr = if self.prg_rom.len() == 0x4000 && addr >= 0x4000 {
            addr % 0x4000
        } else {
            addr
        };
        self.prg_rom[effective_addr as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bus_fetch_and_store_byte() {
        let rom = Rom::empty();
        let mut bus = Bus::new(rom);

        // Store a byte and verify retrieval
        bus.store_byte(5, 42);
        assert_eq!(bus.fetch_byte(5), 42);
    }
}
