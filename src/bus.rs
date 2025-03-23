use crate::memory::heap_memory::HeapMemory;
use crate::memory::memory_trait::MemoryTrait;

const ROM_SIZE: usize = 1<<16;
pub struct Bus {
    memory: HeapMemory<u8>,
}

impl Bus {
    pub fn new() -> Self {
        Self {
            memory: HeapMemory::new(ROM_SIZE, 0u8)
        }
    }

    pub fn fetch_byte(&self, address: usize) -> u8 {
        *self.memory.read(address)
    }

    pub fn store_byte(&mut self, address: usize, value: u8) {
        self.memory.write(address, value);
    }

    pub fn store_bytes(&mut self, address: usize, values: &[u8]) {
        self.memory.write_n(address, values);
    }

    pub fn store_byte_vec(&mut self, address: usize, values: Vec<u8>) {
        self.memory.write_n(address, &values.into_boxed_slice())
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

    #[test]
    #[should_panic]
    fn test_bus_fetch_out_of_bounds() {
        let bus = Bus::new();
        let _ = bus.fetch_byte(ROM_SIZE+1); // Should panic (out-of-bounds)
    }

    #[test]
    #[should_panic]
    fn test_bus_store_out_of_bounds() {
        let mut bus = Bus::new();
        bus.store_byte(ROM_SIZE+1, 100); // Should panic (out-of-bounds)
    }
}
