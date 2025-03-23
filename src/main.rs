mod memory;
mod bus;
mod cpu;

use bus::Bus;
use cpu::CPU;
use memory::heap_memory::HeapMemory;
use memory::memory_trait::MemoryTrait;
use memory::stack_memory::StackMemory;

fn main() {
    // Create HeapMemory for the ROM
    let heap_mem = HeapMemory::new(256, 0u8); // 256-byte memory initialized to 0

    // Create StackMemory for the cpu RAM
    let mut stack_mem = StackMemory::<u8, 256>::new(256, 0u8);

    // Create a Bus instance
    let mut bus = Bus::new(heap_mem);

    // Store some values
    bus.store_byte(0x10, 42);
    stack_mem.write_n(0x10, &[1, 3, 3, 7]);

    // Fetch the values
    let value = bus.fetch_byte(0x10);
    println!("Fetched value from bus heap at $10: {}", value);
    println!("Fetched 10 values from stack starting at $0D: {:?}", stack_mem.read_n(0x0D, 10));


    let cpu = CPU::new(bus);
}
