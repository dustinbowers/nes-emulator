use crate::memory::heap_memory::HeapMemory;
use crate::memory::memory_trait::MemoryTrait;
use crate::rom::Rom;
use macroquad::telemetry::disable;

const CPU_RAM_SIZE: usize = 1 << 11;
const CPU_RAM_START: u16 = 0x0000;
const CPU_RAM_END: u16 = 0x1FFF;
const CPU_MIRROR_MASK: u16 = 0b0001_1111_1111_1111;

const PPU_REGISTERS_START: u16 = 0x2000;
const PPU_REGISTERS_END: u16 = 0x3FFF;
const PPU_MIRROR_MASK: u16 = 0b0010_0000_0000_0111;

pub struct Bus {
    pub cpu_ram: HeapMemory<u8>,
    pub cycles: usize,
    rom: Rom,
    pub disable_mirroring: bool,
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
            PPU_REGISTERS_START..=PPU_REGISTERS_END => {
                let _mirrored_address = address & PPU_MIRROR_MASK;
                todo!("PPU registers not available");
            }
            _ => todo!(),
        }
    }
    fn store_byte(&mut self, address: u16, value: u8) {
        if self.disable_mirroring {
            self.cpu_ram.write(address as usize, value);
            return;
        }
        match address {
            CPU_RAM_START..=CPU_RAM_END => {
                let mirrored_address = address & CPU_MIRROR_MASK;
                self.cpu_ram.write(mirrored_address as usize, value);
            }
            PPU_REGISTERS_START..=PPU_REGISTERS_END => {
                let _mirrored_address = address & PPU_MIRROR_MASK;
                todo!("PPU registers not available");
            }
            _ => todo!(),
        }
    }
}

impl Bus {
    pub fn new(rom: Rom) -> Bus {
        Self {
            cpu_ram: HeapMemory::new(CPU_RAM_SIZE, 0u8),
            cycles: 0,
            rom,
            disable_mirroring: false,
        }
    }

    pub fn enable_test_mode(&mut self) {
        self.disable_mirroring = true;
        self.cpu_ram.data = std::mem::take(&mut self.rom.prg_rom);
        self.cpu_ram.data.resize(1 << 16, 0u8);
    }

    pub fn tick(&mut self, cycles: usize) {
        self.cycles += cycles;
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
