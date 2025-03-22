mod memory;

use crate::memory::Memory;

// mod emulator;

fn main() {
    println!("Hello, world!");

    // let mut m = Memory::new(1<<16);
    // let mut m: Memory<u8, { 1 << 16 }> = Memory::new(0);
    let mut m = Memory::new({1<<16}, 0u8);

    println!("memory size = {}", m.get_size());

    m.write(0, 127);
    println!("memory at $0 = {}", m.read(0));

    println!("bytes at $0 = {:?}", m.read_n(0, 3));

    m.write_vec(0xF, vec![1, 3, 3, 7]);
    println!("bytes at $0xF = {:?}", m.read_n(0x0E, 6));


}
