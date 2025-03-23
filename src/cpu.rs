use crate::memory::memory_trait::MemoryTrait;

use crate::Bus;

pub struct CPU<M: MemoryTrait<u8>> {
    bus: Bus<M>,
}

impl<M: MemoryTrait<u8>> CPU<M> {
    pub fn new(bus: Bus<M>) -> Self {
        Self { bus }
    }

    // pub fn fetch_byte(&self, address: usize) -> u8 {
    //     *self.memory.read(address)
    // }
    //
    // pub fn store_byte(&mut self, address: usize, value: u8) {
    //     self.memory.write(address, value);
    // }
}
