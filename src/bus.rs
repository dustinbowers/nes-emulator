use crate::memory::memory_trait::MemoryTrait;

pub struct Bus<M: MemoryTrait<u8>> {
    memory: M,
}

impl<M: MemoryTrait<u8>> Bus<M> {
    pub fn new(memory: M) -> Self {
        Self { memory }
    }

    pub fn fetch_byte(&self, address: usize) -> u8 {
        *self.memory.read(address)
    }

    pub fn store_byte(&mut self, address: usize, value: u8) {
        self.memory.write(address, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::stack_memory::StackMemory; // Import a concrete Memory implementation

    #[test]
    fn test_bus_fetch_and_store_byte() {
        let memory = StackMemory::<u8, 16>::new(16, 0);
        let mut bus = Bus::new(memory);

        // Store a byte and verify retrieval
        bus.store_byte(5, 42);
        assert_eq!(bus.fetch_byte(5), 42);
    }

    #[test]
    #[should_panic]
    fn test_bus_fetch_out_of_bounds() {
        let memory = StackMemory::<u8, 16>::new(16, 0);
        let bus = Bus::new(memory);

        let _ = bus.fetch_byte(16); // Should panic (out-of-bounds)
    }

    #[test]
    #[should_panic]
    fn test_bus_store_out_of_bounds() {
        let memory = StackMemory::<u8, 16>::new(16, 0);
        let mut bus = Bus::new(memory);

        bus.store_byte(16, 100); // Should panic (out-of-bounds)
    }
}
