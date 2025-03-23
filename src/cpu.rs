use crate::memory::memory_trait::MemoryTrait;
use crate::memory::stack_memory::StackMemory;
use crate::Bus;

const CPU_RAM_SIZE: usize = 2048;

pub struct CPU<M: MemoryTrait<u8>> {
    cpu_ram: StackMemory<u8, CPU_RAM_SIZE>,
    bus: Bus<M>,
}

impl<M: MemoryTrait<u8>> CPU<M> {
    pub fn new(bus: Bus<M>) -> Self {
        let mut s = Self {
            cpu_ram: StackMemory::new(CPU_RAM_SIZE, 0),
            bus,
        };
        s.cpu_ram.write(0xFF, 55);
        println!("creating CPU. memory at $0xFF = {}", s.cpu_ram.read(0xFF));
        s
    }

    pub fn fetch_byte(&self, address: usize) -> u8 {
        self.bus.fetch_byte(address)
    }

    pub fn store_byte(&mut self, address: usize, value: u8) {
        self.bus.store_byte(address, value);
    }
}
