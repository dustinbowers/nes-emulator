mod memory;

use crate::memory::Memory;

// mod emulator;

fn main() {
    println!("Hello, world!");

    let mut m = Memory::new(1<<16);

    println!("memory size = {}", m.get_size());

    m.write(0, 127);
    println!("memory at $0 = {}", m.read(0))

}
