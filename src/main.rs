mod bus;
mod cpu;
mod memory;

use bus::Bus;
use cpu::CPU;
use memory::memory_trait::MemoryTrait;

fn main() {
    // Create the Bus
    let mut bus = Bus::new();

    // Store a value in the bus
    bus.store_byte(0x10, 42);

    // Fetch the value
    let value = bus.fetch_byte(0x10);
    println!("Fetched value from bus heap at $10: {}", value);
    assert_eq!(value, 42);

    // Create a CPU and store a byte to ROM through the bus
    let mut cpu = CPU::new(bus);
    cpu.store_byte(0x10, 84);

    // Make the CPU fetch the value from ROM through the bus
    let value = cpu.fetch_byte(0x10);
    println!("Fetched value from cpu at $10: {}", value);
    assert_eq!(value, 84);
}
