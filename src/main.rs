mod bus;
mod cpu;
mod memory;

use bus::Bus;
use cpu::CPU;
use memory::heap_memory::HeapMemory;
use memory::memory_trait::MemoryTrait;

fn main() {
    // Create HeapMemory for the ROM
    let heap_mem = HeapMemory::new(256, 0u8); // 256-byte memory initialized to 0

    // Create the Bus
    let mut bus = Bus::new(heap_mem);

    // Store a value through the bus
    bus.store_byte(0x10, 42);

    // Fetch the value
    let value = bus.fetch_byte(0x10);
    println!("Fetched value from bus heap at $10: {}", value);
    assert_eq!(value, 42);

    let mut cpu = CPU::new(bus);
    cpu.store_byte(0x10, 84);
    let value = cpu.fetch_byte(0x10);
    println!("Fetched value from cpu at $10: {}", value);
    assert_eq!(value, 84);
}
