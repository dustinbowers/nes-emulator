use bitflags::bitflags;
use crate::memory::memory_trait::MemoryTrait;
use crate::memory::stack_memory::StackMemory;
use crate::Bus;

const CPU_RAM_SIZE: usize = 2048;

bitflags! {
    pub struct Flags: u8 {
        const CARRY             = 1<<0;
        const ZERO              = 1<<1;
        const INTERRUPT_DISABLE = 1<<2;
        const DECIMAL_MODE      = 1<<3;
        const BREAK             = 1<<4;
        const BREAK2            = 1<<5;
        const OVERFLOW          = 1<<6;
        const NEGATIVE          = 1<<7;
    }
}
pub struct CPU {
    cpu_ram: StackMemory<u8, CPU_RAM_SIZE>,
    bus: Bus,

    register_a: u8,
    register_b: u8,
    register_x: u8,
    status: Flags,
    program_counter: u16,
}

impl CPU {
    pub fn new(bus: Bus) -> Self {
        let mut s = Self {
            cpu_ram: StackMemory::new(CPU_RAM_SIZE, 0),
            bus,
            register_a: 0,
            register_b: 0,
            register_x: 0,
            status: Flags::from_bits_truncate(0b0000_0010),
            program_counter: 0,
        };
        s
    }

    pub fn fetch_byte(&self, address: usize) -> u8 {
        self.bus.fetch_byte(address)
    }

    pub fn store_byte(&mut self, address: usize, value: u8) {
        self.bus.store_byte(address, value);
    }

    pub fn run(&mut self) {
        self.program_counter = 0;

        loop {
            let opcode = self.fetch_byte(self.program_counter as usize);
            self.program_counter += 1;

            match opcode {
                0xA9 => { // LDA
                    let param = self.fetch_byte(self.program_counter as usize);
                    self.program_counter += 1;
                    self.lda(param);
                }
                0xAA => self.tax(), // TAX
                0x00 => return, // BRK
                _ => todo!()
            }
        }
    }

    fn lda(&mut self, value: u8) {
        self.register_a = value;
        self.update_zero_and_negative_flags(self.register_a);
    }

    fn tax(&mut self) {
        self.register_x = self.register_a;
        self.update_zero_and_negative_flags(self.register_x);
    }


    fn update_zero_and_negative_flags(&mut self, result: u8) {
        self.status.set(Flags::ZERO, result == 0);
        self.status.set(Flags::NEGATIVE, result & 0b1000_0000 != 0);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn init_cpu() -> CPU {
        let bus = Bus::new();
        CPU::new(bus)
    }
    
    #[test]
    fn test_0xa9_lda_immediate_load_data() {
        let mut cpu = init_cpu();
        let program = &[
            0xa9, // LDA immediate
            0x05, //    with $05
            0x00, // BRK
        ];
        cpu.bus.store_bytes(0x0, program);
        cpu.run();
        assert_eq!(cpu.register_a, 0x05);
        assert_eq!(cpu.status.contains(Flags::ZERO), false);
        assert_eq!(cpu.status.contains(Flags::NEGATIVE), false);
    }

    #[test]
    fn test_0xa9_lda_zero_flag() {
        let mut cpu = init_cpu();
        let program = &[
            0xa9, // LDA immediate
            0x00, //    with $0
            0x00, // BRK
        ];
        cpu.bus.store_bytes(0x0, program);
        cpu.run();
        assert_eq!(cpu.status.contains(Flags::ZERO), true);
    }
}