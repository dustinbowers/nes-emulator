use crate::{opcodes, Bus};
use bitflags::bitflags;
use std::collections::HashMap;

const DEBUG: bool = true;

const CPU_RAM_SIZE: usize = 2048;
const CPU_PC_RESET: u16 = 0x8000;
const CPU_STACK_RESET: u8 = 0x00FD;
const CPU_STACK_BASE: u16 = (CPU_STACK_RESET as u16) + 0x2;

bitflags! {
    /* https://www.nesdev.org/wiki/Status_flags
            7  bit  0
        ---- ----
        NV1B DIZC
        |||| ||||
        |||| |||+- Carry
        |||| ||+-- Zero
        |||| |+--- Interrupt Disable
        |||| +---- Decimal
        |||+------ (No CPU effect; see: the B flag)
        ||+------- (No CPU effect; always pushed as 1)
        |+-------- Overflow
        +--------- Negative
     */
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
            let opcode = *opcodes
                .get(&code)
                .expect(&format!("Unknown opcode: {:#x}", &code));
            let curr_program_counter = self.program_counter;

            if DEBUG {
                println!(
                    "PC:{:#x} SP:{:#x} A:{:#x} X:{:#x} Y:{:#x} - Opcode: {} {:x?}",
                    self.program_counter,
                    self.stack_pointer,
                    self.register_a,
                    self.register_x,
                    self.register_y,
                    opcode.name,
                    self.bus.fetch_bytes(self.program_counter - 1, opcode.size)
                )
            }

            match code {
                0x00 => return, // BRK
                0xEA => {}      // NOP

                0xAA => self.tax(), // TAX
                0xA8 => self.tay(), // TAY
                0xBA => self.tsx(), // TSX
                0x8A => self.txa(), // TXA
                0x9A => self.txs(), // TXS
                0x98 => self.tya(), // TYA

                0xD8 => self.cld(), // CLD
                0x58 => self.cli(), // CLI
                0xB8 => self.clv(), // CLV
                0x18 => self.clc(), // CLC
                0x38 => self.sec(), // SEC
                0x78 => self.sei(), // SEI
                0xF8 => self.sed(), // SED

                0xE8 => self.inx(), // INX
                0xC8 => self.iny(), // INY

                0xCA => self.dex(), // DEX
                0x88 => self.dey(), // DEY

                0x48 => self.pha(), // PHA
                0x68 => self.pla(), // PLA
                0x08 => self.php(), // PHP
                0x28 => self.plp(), // PLP

                0xA9 | 0xA5 | 0xB5 | 0xAD | 0xBD | 0xB9 | 0xA1 | 0xB1 => {
                    self.lda(opcode); // LDA
                }
                0xA2 | 0xA6 | 0xB6 | 0xAE | 0xBE => {
                    self.ldx(opcode); // LDX
                }
                0xA0 | 0xA4 | 0xB4 | 0xAC | 0xBC => {
                    self.ldy(opcode); // LDY
                }
                0x85 | 0x95 | 0x8D | 0x9D | 0x99 | 0x81 | 0x91 => {
                    self.sta(opcode); // STA
                }
                0x86 | 0x96 | 0x8e => {
                    self.stx(opcode); // STX
                }
                0x84 | 0x94 | 0x8c => {
                    self.sty(opcode); // STY
                }
                0x0A | 0x06 | 0x16 | 0x0E | 0x1E => {
                    self.asl(opcode); // ASL
                }
                0x4A | 0x46 | 0x56 | 0x4E | 0x5E => {
                    self.lsr(opcode); // LSR
                }
                0x2A | 0x26 | 0x36 | 0x2E | 0x3E => {
                    self.rol(opcode); // ROL
                }
                0x6A | 0x66 | 0x76 | 0x6E | 0x73 => {
                    self.ror(opcode); // ROR
                }
                0xE6 | 0xF6 | 0xEE | 0xFE => {
                    self.inc(opcode); // INC
                }
                0xC6 | 0xD6 | 0xCE | 0xDE => {
                    self.dec(opcode); // DEC
                }
                0xC9 | 0xC5 | 0xD5 | 0xCD | 0xDD | 0xD9 | 0xC1 | 0xD1 => {
                    self.cmp(opcode); // CMP
                }
                0xE0 | 0xE4 | 0xEC => {
                    self.cpx(opcode); // CPX
                }
                0xC0 | 0xC4 | 0xCC => {
                    self.cpy(opcode); // CPY
                }
                0x69 | 0x65 | 0x75 | 0x6D | 0x7D | 0x79 | 0x61 | 0x71 => {
                    self.adc(opcode); // ADC
                }
                0xE9 | 0xE5 | 0xF5 | 0xED | 0xFD | 0xF9 | 0xE1 | 0xF1 => {
                    self.sbc(opcode); // SBC
                }
                0x29 | 0x25 | 0x35 | 0x2D | 0x3D | 0x39 | 0x21 | 0x31 => {
                    self.and(opcode); // AND
                }
                0x49 | 0x45 | 0x55 | 0x4D | 0x5D | 0x59 | 0x41 | 0x51 => {
                    self.eor(opcode); // EOR
                }

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
            _ => unimplemented!("Unimplemented addressing mode"),
        }
    }

    // Utility functions
    /////////////////////
    fn set_register_a(&mut self, value: u8) {
        self.register_a = value;
        self.update_zero_and_negative_flags(value);
    }

    fn set_register_x(&mut self, value: u8) {
        self.register_x = value;
        self.update_zero_and_negative_flags(value);
    }

    fn set_register_y(&mut self, value: u8) {
        self.register_y = value;
        self.update_zero_and_negative_flags(value);
    }

    fn stack_push(&mut self, value: u8) {
        self.bus
            .store_byte(CPU_STACK_BASE + self.stack_pointer as u16, value);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
    }

    fn stack_pop(&mut self) -> u8 {
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        self.bus
            .fetch_byte(CPU_STACK_BASE + self.stack_pointer as u16)
    }

    fn update_zero_and_negative_flags(&mut self, result: u8) {
        self.status.set(Flags::ZERO, result == 0);
        self.status.set(Flags::NEGATIVE, result & 0b1000_0000 != 0);
    }

    // Opcodes
    /////////////
    fn lda(&mut self, opcode: &opcodes::Opcode) {
        let (address, boundary_cross) = self.get_parameter_address(&opcode.mode);
        self.extra_cycles += boundary_cross as u8; // boundary_cross adds 1 extra cycle

        let param = self.fetch_byte(address);
        self.set_register_a(param);
    }

    fn ldx(&mut self, opcode: &opcodes::Opcode) {
        let (address, boundary_cross) = self.get_parameter_address(&opcode.mode);
        self.extra_cycles += boundary_cross as u8; // boundary_cross adds 1 extra cycle

        let param = self.fetch_byte(address);
        self.set_register_x(param);
    }

    fn ldy(&mut self, opcode: &opcodes::Opcode) {
        let (address, boundary_cross) = self.get_parameter_address(&opcode.mode);
        self.extra_cycles += boundary_cross as u8; // boundary_cross adds 1 extra cycle

        let param = self.fetch_byte(address);
        self.set_register_y(param);
    }

    fn sta(&mut self, opcode: &opcodes::Opcode) {
        let (address, boundary_cross) = self.get_parameter_address(&opcode.mode);
        self.bus.store_byte(address, self.register_a);
    }

    fn stx(&mut self, opcode: &opcodes::Opcode) {
        let (address, boundary_cross) = self.get_parameter_address(&opcode.mode);
        self.bus.store_byte(address, self.register_x);
    }

    fn sty(&mut self, opcode: &opcodes::Opcode) {
        let (address, boundary_cross) = self.get_parameter_address(&opcode.mode);
        self.bus.store_byte(address, self.register_y);
    }

    fn tax(&mut self) {
        self.set_register_x(self.register_a);
    }

    fn tay(&mut self) {
        self.set_register_y(self.register_a);
    }

    fn tsx(&mut self) {
        self.set_register_x(self.stack_pointer);
    }

    fn txa(&mut self) {
        self.set_register_a(self.register_x);
    }

    fn txs(&mut self) {
        self.stack_pointer = self.register_x;
    }

    fn tya(&mut self) {
        self.set_register_a(self.register_y);
    }

    fn cld(&mut self) {
        self.status.remove(Flags::DECIMAL_MODE);
    }

    fn cli(&mut self) {
        self.status.remove(Flags::INTERRUPT_DISABLE);
    }

    fn clv(&mut self) {
        self.status.remove(Flags::OVERFLOW);
    }

    fn clc(&mut self) {
        self.status.remove(Flags::CARRY);
    }

    fn sec(&mut self) {
        self.status.insert(Flags::CARRY);
    }

    fn sei(&mut self) {
        self.status.insert(Flags::INTERRUPT_DISABLE);
    }

    fn sed(&mut self) {
        self.status.insert(Flags::DECIMAL_MODE);
    }

    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_x);
    }

    fn iny(&mut self) {
        self.register_y = self.register_y.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_y);
    }

    fn dex(&mut self) {
        self.register_x = self.register_x.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.register_x);
    }

    fn dey(&mut self) {
        self.register_y = self.register_y.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.register_y);
    }

    fn pha(&mut self) {
        // Push register_a onto the stack
        self.stack_push(self.register_a)
    }

    fn pla(&mut self) {
        // Pop stack into register_a
        let value = self.stack_pop();
        self.set_register_a(value);
    }

    fn php(&mut self) {
        // Push processor_status onto the stack
        // https://www.nesdev.org/wiki/Status_flags
        // says that B flag is pushed as 1, but not affected on the CPU
        let mut status_copy = Flags::from_bits_truncate(self.status.bits());
        status_copy.insert(Flags::BREAK);
        self.stack_push(status_copy.bits())
    }

    fn plp(&mut self) {
        // Pop stack into processor_status
        self.status = Flags::from_bits_truncate(self.stack_pop());
        self.status.insert(Flags::BREAK2); // This flag is supposed to always be 1 on CPU
    }

    fn asl(&mut self, opcode: &opcodes::Opcode) {
        // Arithmetic Shift Left into carry
        let (address, _) = self.get_parameter_address(&opcode.mode);
        let mut value = self.bus.fetch_byte(address);
        let carry = value & 0b1000_0000 != 0;
        value <<= 1;
        self.bus.store_byte(address, value);
        self.status.set(Flags::CARRY, carry);
        self.update_zero_and_negative_flags(value);
    }

    fn lsr(&mut self, opcode: &opcodes::Opcode) {
        // Logical Shift Right into carry
        let (address, _) = self.get_parameter_address(&opcode.mode);
        let mut value = self.bus.fetch_byte(address);
        let carry = value & 0b1 != 0;
        value >>= 1;
        self.bus.store_byte(address, value);
        self.status.set(Flags::CARRY, carry);
        self.update_zero_and_negative_flags(value);
    }

    fn rol(&mut self, opcode: &opcodes::Opcode) {
        // Rotate Left through carry flag
        let (address, _) = self.get_parameter_address(&opcode.mode);
        let mut value = self.bus.fetch_byte(address);
        let prev_carry = self.status.contains(Flags::CARRY);
        let new_carry = value & 0b1000_0000 != 0;
        value <<= 1;
        if prev_carry {
            value |= 1;
        }
        self.bus.store_byte(address, value);
        self.status.set(Flags::CARRY, new_carry);
    }

    fn ror(&mut self, opcode: &opcodes::Opcode) {
        // Rotate Right through carry flag
        let (address, _) = self.get_parameter_address(&opcode.mode);
        let mut value = self.bus.fetch_byte(address);
        let prev_carry = self.status.contains(Flags::CARRY);
        let new_carry = value & 0b0000_0001 != 0;
        value >>= 1;
        if prev_carry {
            value |= 0b1000_0000;
        }
        self.bus.store_byte(address, value);
        self.status.set(Flags::CARRY, new_carry);
    }

    fn inc(&mut self, opcode: &opcodes::Opcode) {
        // Increment value at Memory
        let (address, _) = self.get_parameter_address(&opcode.mode);
        let mut value = self.bus.fetch_byte(address);
        value = value.wrapping_add(1);
        self.bus.store_byte(address, value);
        self.update_zero_and_negative_flags(value);
    }

    fn dec(&mut self, opcode: &opcodes::Opcode) {
        // Decrement value at Memory
        let (address, _) = self.get_parameter_address(&opcode.mode);
        let mut value = self.bus.fetch_byte(address);
        value = value.wrapping_sub(1);
        self.bus.store_byte(address, value);
        self.update_zero_and_negative_flags(value);
    }

    fn compare(&mut self, opcode: &opcodes::Opcode, compare_value: u8) {
        let (address, boundary_crossed) = self.get_parameter_address(&opcode.mode);
        let value = self.bus.fetch_byte(address);
        self.status.set(Flags::CARRY, compare_value >= value);
        self.update_zero_and_negative_flags(compare_value.wrapping_sub(value));
        self.extra_cycles += boundary_crossed as u8;
    }

    fn cmp(&mut self, opcode: &opcodes::Opcode) {
        // Compare A register
        self.compare(opcode, self.register_a);
    }

    fn cpx(&mut self, opcode: &opcodes::Opcode) {
        // Compare X Register
        self.compare(opcode, self.register_x);
    }

    fn cpy(&mut self, opcode: &opcodes::Opcode) {
        // Compare Y Register
        self.compare(opcode, self.register_y);
    }

    fn add_to_register_a(&mut self, value: u8) {
        // TODO: Test this...
        let curr_carry = self.status.contains(Flags::CARRY) as u8;
        let result = self.register_a.wrapping_add(value + curr_carry);

        // Method: OVERFLOW if the sign of the inputs are the same,
        //         and do not match the sign of the result
        // Reasoning: A signed overflow MUST have occurred in these cases:
        //              * Positive + Positive = Negative OR
        //              * Negative + Negative = Positive
        // Boolean logic: (!((register_a ^ value) & 0x80) && ((register_a ^ result) & 0x80))
        // See: https://forums.nesdev.org/viewtopic.php?t=6331
        let signed_overflow = !((self.register_a ^ value) & 0x80 != 0) && ((self.register_a ^ result) & 0x80 != 0);
        self.status.set(Flags::OVERFLOW, signed_overflow);

        self.set_register_a(result);
    }

    fn sub_from_register_a(&mut self, data: u8) {
        self.add_to_register_a(!data);
    }

    fn adc(&mut self, opcode: &opcodes::Opcode) {
        // Add with Carry
        let (address, boundary_crossed) = self.get_parameter_address(&opcode.mode);
        let value = self.bus.fetch_byte(address);
        self.add_to_register_a(value);
        self.extra_cycles += boundary_crossed as u8;
    }

    fn sbc(&mut self, opcode: &opcodes::Opcode) {
        // Subtract with Carry
        let (address, boundary_crossed) = self.get_parameter_address(&opcode.mode);
        let value = self.bus.fetch_byte(address);
        self.sub_from_register_a(value);
        self.extra_cycles += boundary_crossed as u8;
    }

    fn and(&mut self, opcode: &opcodes::Opcode) {
        // Logical AND on accumulator
        let (address, boundary_crossed) = self.get_parameter_address(&opcode.mode);
        let value = self.bus.fetch_byte(address);
        self.set_register_a(self.register_a & value);
        self.extra_cycles += boundary_crossed as u8;
    }

    fn eor(&mut self, opcode: &opcodes::Opcode) {
        // Logical Exclusive OR on accumulator
        let (address, boundary_crossed) = self.get_parameter_address(&opcode.mode);
        let value = self.bus.fetch_byte(address);
        self.set_register_a(self.register_a ^ value);
        self.extra_cycles += boundary_crossed as u8;
    }

    fn ora(&mut self, opcode: &opcodes::Opcode) {
        // Logical OR on accumulator
        let (address, boundary_crossed) = self.get_parameter_address(&opcode.mode);
        let value = self.bus.fetch_byte(address);
        self.set_register_a(self.register_a | value);
        self.extra_cycles += boundary_crossed as u8;
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

    #[test]
    fn test_0xb5_lda_absolute_load_data() {
        let mut cpu = init_cpu();
        let program = &[
            0xAD, // LDA absolute (5 cycles)
            0xEF, //
            0xBE, // Loading from little endian $EFBE which will actually be $BEEF
            0xAA, // TAX (1 cycle)
            0x00, // BRK
        ];
        cpu.load(program);
        cpu.bus.store_byte(0xBEEF, 0x42);
        cpu.run();
        assert_eq!(cpu.register_a, 0x42);
        assert_eq!(cpu.register_x, 0x42);
        assert_eq!(cpu.bus.cycles, 5 + 1);
        assert_eq!(cpu.status.contains(Flags::ZERO), false);
        assert_eq!(cpu.status.contains(Flags::NEGATIVE), false);
    }

    #[test]
    fn test_set_flags() {
        let mut cpu = init_cpu();
        let program = &[
            0x38, // SEC
            0x78, // SEI
            0xF8, // SED
            0x00, // BRK
        ];
        cpu.load(program);
        cpu.run();
        assert_eq!(cpu.status.contains(Flags::CARRY), true);
        assert_eq!(cpu.status.contains(Flags::INTERRUPT_DISABLE), true);
        assert_eq!(cpu.status.contains(Flags::DECIMAL_MODE), true);
    }

    #[test]
    fn test_set_and_clear_flags() {
        let mut cpu = init_cpu();
        let program = &[
            0x38, // SEC
            0x78, // SEI
            0xF8, // SED
            0x18, // CLC
            0x58, // CLI
            0xD8, // CLD
            0x00, // BRK
        ];
        cpu.load(program);
        cpu.run();
        assert_eq!(cpu.status.contains(Flags::CARRY), false);
        assert_eq!(cpu.status.contains(Flags::INTERRUPT_DISABLE), false);
        assert_eq!(cpu.status.contains(Flags::DECIMAL_MODE), false);
    }
}
