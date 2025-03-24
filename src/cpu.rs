use crate::{opcodes, Bus};
use bitflags::bitflags;
use std::collections::HashMap;

const CPU_RAM_SIZE: usize = 2048;
const CPU_PC_RESET: u16 = 0x8000;
const CPU_STACK_RESET: u8 = 0x00FD;
const CPU_STACK_BASE: u16 = (CPU_STACK_RESET as u16) + 0x2;

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

pub enum AddressingMode {
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    IndirectX,
    IndirectY,
    None,
}

pub struct CPU {
    bus: Bus,

    register_a: u8,
    register_x: u8,
    register_y: u8,
    stack_pointer: u8,
    status: Flags,
    program_counter: u16,

    extra_cycles: u8,
}

impl CPU {
    pub fn new(bus: Bus) -> Self {
         Self {
            bus,
            register_a: 0,
            register_x: 0,
            register_y: 0,
            stack_pointer: 0,
            status: Flags::from_bits_truncate(0b0000_0010),
            program_counter: CPU_PC_RESET,
            extra_cycles: 0,
        }
    }

    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.program_counter = CPU_PC_RESET;
        self.status = Flags::from_bits_truncate(0b0000_0010);
        self.extra_cycles = 0;
    }

    pub fn load(&mut self, program: &[u8]) {
        self.reset();
        self.bus.store_bytes(self.program_counter, program);
    }

    pub fn fetch_byte(&mut self, address: u16) -> u8 {
        self.bus.fetch_byte(address)
    }

    pub fn fetch_u16(&mut self, address: u16) -> u16 {
        let lo = self.bus.fetch_byte(address) as u16;
        let hi = self.bus.fetch_byte(address + 1) as u16;
        hi << 8 | lo
    }

    pub fn store_byte(&mut self, address: u16, value: u8) {
        self.bus.store_byte(address, value);
    }

    pub fn run(&mut self) {
        let ref opcodes: HashMap<u8, &'static opcodes::Opcode> = *opcodes::OPCODES_MAP;

        loop {
            self.extra_cycles = 0;
            let code = self.fetch_byte(self.program_counter);
            self.program_counter += 1;
            let opcode = *opcodes.get(&code).expect("");
            let curr_program_counter = self.program_counter;

            match code {
                0xA9 | 0xA5 | 0xB5 | 0xAD | 0xBD | 0xB9 | 0xA1 | 0xB1 => {
                    // LDA
                    self.lda(opcode);
                }
                0xAA => self.tax(), // TAX
                0x00 => return,     // BRK
                _ => todo!(),
            }

            // Tick the bus for opcode cycles. Add any extra cycles from boundary_crosses and other special cases
            let cycle_count = opcode.cycles + self.extra_cycles;
            self.bus.tick(cycle_count as usize);

            // If the opcode didn't move PC by some call/ret/branch, then
            // we step it forward by the size of the opcode - 1
            // since we've already stepped it forward one byte when reading it
            if curr_program_counter == self.program_counter {
                self.program_counter += (opcode.size - 1) as u16;
            }
        }
    }

    fn get_parameter_address(&mut self, mode: &AddressingMode) -> (u16, bool) {
        match mode {
            AddressingMode::Absolute => (self.fetch_u16(self.program_counter), false),
            AddressingMode::Immediate => (self.program_counter, false),
            AddressingMode::ZeroPage => (self.fetch_byte(self.program_counter) as u16, false),
            AddressingMode::ZeroPageX => {
                let base = self.fetch_byte(self.program_counter);
                let addr = base.wrapping_add(self.register_x) as u16;
                (addr, false)
            }
            AddressingMode::ZeroPageY => {
                let base = self.fetch_byte(self.program_counter);
                let addr = base.wrapping_add(self.register_y) as u16;
                (addr, false)
            }
            AddressingMode::AbsoluteX => {
                let base = self.fetch_u16(self.program_counter);
                let addr = base.wrapping_add(self.register_x as u16);
                (addr, is_boundary_crossed(base, addr))
            }
            AddressingMode::AbsoluteY => {
                let base = self.fetch_u16(self.program_counter);
                let addr = base.wrapping_add(self.register_y as u16);
                (addr, is_boundary_crossed(base, addr))
            }
            AddressingMode::IndirectX => {
                let base = self.fetch_byte(self.program_counter);
                let addr = base.wrapping_add(self.register_x); // Zero-page wrapping
                let lo = self.fetch_byte(addr as u16) as u16;
                let hi = self.fetch_byte(addr.wrapping_add(1) as u16) as u16; // Zero-page wrap +1 as well
                (hi << 8 | lo, false)
            }
            AddressingMode::IndirectY => {
                let base = self.fetch_byte(self.program_counter) as u16;
                let lo = self.fetch_byte(base) as u16;
                let hi = self.fetch_byte((base as u8).wrapping_add(1) as u16) as u16;
                let dynamic_base = hi << 8 | lo;
                let addr = dynamic_base.wrapping_add(self.register_y as u16);
                (addr, is_boundary_crossed(dynamic_base, addr))
            }
            _ => unimplemented!("Unimplemented addressing mode")
        }
    }

    fn set_register_a(&mut self, value: u8) {
        self.register_a = value;
        self.update_zero_and_negative_flags(value);
    }

    fn set_register_x(&mut self, value: u8) {
        self.register_x = value;
        self.update_zero_and_negative_flags(value);
    }

    fn lda(&mut self, opcode: &opcodes::Opcode) {
        let (address, boundary_cross) = self.get_parameter_address(&opcode.mode);
        self.extra_cycles += boundary_cross as u8; // boundary_cross adds 1 extra cycle
        let param = self.fetch_byte(address);

        self.set_register_a(param);
    }

    fn tax(&mut self) {
        self.set_register_x(self.register_a);
    }

    fn update_zero_and_negative_flags(&mut self, result: u8) {
        self.status.set(Flags::ZERO, result == 0);
        self.status.set(Flags::NEGATIVE, result & 0b1000_0000 != 0);
    }
}

fn is_boundary_crossed(addr1: u16, addr2: u16) -> bool {
    addr1 & 0xFF00 != addr2 & 0xFF00
}

#[cfg(test)]
mod test {
    use super::*;

    fn init_cpu() -> CPU {
        let bus = Bus::new();
        CPU::new(bus)
    }

    #[test]
    fn test_0xaa_tax() {
        let mut cpu = init_cpu();
        let program = &[
            0xa9, // LDA immediate
            0x42, //    with $0F
            0xAA, // TAX
            0x00, // BRK
        ];
        cpu.load(program);
        cpu.run();
        assert_eq!(cpu.register_x, 0x42);
        assert_eq!(cpu.status.contains(Flags::ZERO), false);
        assert_eq!(cpu.status.contains(Flags::NEGATIVE), false);
    }

    #[test]
    fn test_0xa9_lda_immediate_load_data() {
        let mut cpu = init_cpu();
        let program = &[
            0xa9, // LDA immediate
            0x05, //    with $05
            0x00, // BRK
        ];
        cpu.load(program);
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
        cpu.load(program);
        cpu.run();
        assert_eq!(cpu.status.contains(Flags::ZERO), true);
    }

    #[test]
    fn test_0xa5_lda_zero_page_load_data() {
        let mut cpu = init_cpu();
        let program = &[
            0xa5, // LDA ZeroPage
            0x05, //    with $05
            0x00, // BRK
        ];
        cpu.load(program);
        cpu.bus.store_byte(0x05, 0x42);
        cpu.run();
        assert_eq!(cpu.register_a, 0x42);
        assert_eq!(cpu.status.contains(Flags::ZERO), false);
        assert_eq!(cpu.status.contains(Flags::NEGATIVE), false);
    }

    #[test]
    fn test_0xa5_lda_zero_page_x_load_data() {
        let mut cpu = init_cpu();
        let program = &[
            0xa9, // LDA immediate
            0x0F, //    with $0F
            0xAA, // TAX
            0xB5, // LDA ZeroPageX
            0x80, //    with $80        - X = $0F, loading A with data from $8F = 0x42
            0x00, // BRK
        ];
        cpu.load(program);
        cpu.bus.store_byte(0x8F, 0x42);
        cpu.run();
        assert_eq!(cpu.register_a, 0x42);
        assert_eq!(cpu.register_x, 0x0F);
        assert_eq!(cpu.status.contains(Flags::ZERO), false);
        assert_eq!(cpu.status.contains(Flags::NEGATIVE), false);
    }
}
