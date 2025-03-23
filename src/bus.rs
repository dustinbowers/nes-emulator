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
