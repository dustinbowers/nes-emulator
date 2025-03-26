use crate::memory::heap_memory::HeapMemory;
use crate::memory::memory_trait::MemoryTrait;
use std::marker::PhantomData;

const ROM_SIZE: usize = 1 << 16;
pub struct Bus {
    pub memory: HeapMemory<u8>,
    pub cycles: usize,
}

impl Bus {
    pub fn new() -> Bus {
        Self {
            memory: HeapMemory::new(ROM_SIZE, 0u8),
            cycles: 0,
        }
    }

    pub fn tick(&mut self, cycles: usize) {
        self.cycles += cycles;
    }

    pub fn fetch_byte(&mut self, address: u16) -> u8 {
        // TODO: impose correct memory mapping / mirroring for NES
        *self.memory.read(address as usize)
    }

    pub fn fetch_bytes(&mut self, address: u16, size: u8) -> &[u8] {
        self.memory.read_n(address as usize, size as usize)
    }

    pub fn fetch_bytes_raw(&mut self, address: u16, size: u16) -> &[u8] {
        self.memory.read_n(address as usize, size as usize)
    }

    pub fn store_byte(&mut self, address: u16, value: u8) {
        self.memory.write(address as usize, value);
    }

    pub fn store_bytes(&mut self, address: u16, values: &[u8]) {
        self.memory.write_n(address as usize, values);
    }

    pub fn store_byte_vec(&mut self, address: u16, values: Vec<u8>) {
        self.memory
            .write_n(address as usize, &values.into_boxed_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bus_fetch_and_store_byte() {
        let mut bus = Bus::new();

        // Store a byte and verify retrieval
        bus.store_byte(5, 42);
        assert_eq!(bus.fetch_byte(5), 42);
    }
}
