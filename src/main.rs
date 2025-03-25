mod bus;
mod cpu;
mod memory;
mod opcodes;

use bus::Bus;
use cpu::CPU;

fn main() {
    // Create the Bus
    let mut bus = Bus::new();

    // Create a CPU
    let mut cpu = CPU::new(bus);

    // TODO
}
