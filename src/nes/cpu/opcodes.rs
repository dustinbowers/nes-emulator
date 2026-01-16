use once_cell::sync::Lazy;
use std::collections::HashMap;
use super::{AccessType, AddressingMode, CPU};

#[derive(Debug)]
pub struct Opcode {
    pub code: u8,
    pub name: &'static str,
    pub cycles: u8,
    pub size: u8,
    pub mode: AddressingMode,
    pub access_type: AccessType,
    pub exec: fn(&mut CPU) -> bool,
}

impl Opcode {
    pub const fn new(
        value: u8,
        name: &'static str,
        size: u8,
        cycles: u8,
        mode: AddressingMode,
        access_type: AccessType,
        exec: fn(&mut CPU) -> bool,
    ) -> Self {
        Self {
            code: value,
            name,
            cycles,
            size,
            mode,
            access_type,
            exec
        }
    }
}



#[rustfmt::skip]
const OPCODES: &[Opcode] = &[
    // Software-defined interrupt
    Opcode::new(0x00, "BRK", 2, 7, AddressingMode::None, AccessType::Read, CPU::brk),

    // General NOP
    Opcode::new(0xEA, "NOP", 1, 2, AddressingMode::None, AccessType::None, CPU::nop),

    // Transfers
    Opcode::new(0xAA, "TAX", 1, 2, AddressingMode::None, AccessType::Register, CPU::tax),
    Opcode::new(0xA8, "TAY", 1, 2, AddressingMode::None, AccessType::Register, CPU::tay),
    Opcode::new(0x8A, "TXA", 1, 2, AddressingMode::None, AccessType::Register, CPU::txa),
    Opcode::new(0x98, "TYA", 1, 2, AddressingMode::None, AccessType::Register, CPU::tya),
    Opcode::new(0xBA, "TSX", 1, 2, AddressingMode::None, AccessType::Register, CPU::tsx),
    Opcode::new(0x9A, "TXS", 1, 2, AddressingMode::None, AccessType::Register, CPU::txs),

    // Flags
    Opcode::new(0xF8, "SED", 1, 2, AddressingMode::None, AccessType::Register, CPU::sed),
    Opcode::new(0x78, "SEI", 1, 2, AddressingMode::None, AccessType::Register, CPU::sei),
    Opcode::new(0x38, "SEC", 1, 2, AddressingMode::None, AccessType::Register, CPU::sec),
    Opcode::new(0xD8, "CLD", 1, 2, AddressingMode::None, AccessType::Register, CPU::cld),
    Opcode::new(0x58, "CLI", 1, 2, AddressingMode::None, AccessType::Register, CPU::cli),
    Opcode::new(0x18, "CLC", 1, 2, AddressingMode::None, AccessType::Register, CPU::clc),
    Opcode::new(0xB8, "CLV", 1, 2, AddressingMode::None, AccessType::Register, CPU::clv),

    //
    // Loads
    Opcode::new(0xA9, "LDA", 2, 2, AddressingMode::Immediate, AccessType::Read, CPU::lda),
    Opcode::new(0xA5, "LDA", 2, 3, AddressingMode::ZeroPage,  AccessType::Read, CPU::lda),
    Opcode::new(0xB5, "LDA", 2, 4, AddressingMode::ZeroPageX, AccessType::Read, CPU::lda),
    Opcode::new(0xAD, "LDA", 3, 4, AddressingMode::Absolute,  AccessType::Read, CPU::lda),
    Opcode::new(0xBD, "LDA", 3, 4, AddressingMode::AbsoluteX, AccessType::Read, CPU::lda), // cycles +1 if page crossed
    Opcode::new(0xB9, "LDA", 3, 4, AddressingMode::AbsoluteY, AccessType::Read, CPU::lda), // cycles +1 if page crossed
    Opcode::new(0xA1, "LDA", 2, 6, AddressingMode::IndirectX, AccessType::Read, CPU::lda),
    Opcode::new(0xB1, "LDA", 2, 5, AddressingMode::IndirectY, AccessType::Read, CPU::lda), // cycles +1 if page crossed

    Opcode::new(0xA2, "LDX", 2, 2, AddressingMode::Immediate, AccessType::Read, CPU::ldx),
    Opcode::new(0xA6, "LDX", 2, 3, AddressingMode::ZeroPage,  AccessType::Read, CPU::ldx),
    Opcode::new(0xB6, "LDX", 2, 4, AddressingMode::ZeroPageY, AccessType::Read, CPU::ldx),
    Opcode::new(0xAE, "LDX", 3, 4, AddressingMode::Absolute,  AccessType::Read, CPU::ldx),
    Opcode::new(0xBE, "LDX", 3, 4, AddressingMode::AbsoluteY, AccessType::Read, CPU::ldx), // cycles +1 if page crossed

    Opcode::new(0xA0, "LDY", 2, 2, AddressingMode::Immediate, AccessType::Read, CPU::ldy),
    Opcode::new(0xA4, "LDY", 2, 3, AddressingMode::ZeroPage,  AccessType::Read, CPU::ldy),
    Opcode::new(0xB4, "LDY", 2, 4, AddressingMode::ZeroPageX, AccessType::Read, CPU::ldy),
    Opcode::new(0xAC, "LDY", 3, 4, AddressingMode::Absolute,  AccessType::Read, CPU::ldy),
    Opcode::new(0xBC, "LDY", 3, 4, AddressingMode::AbsoluteX, AccessType::Read, CPU::ldy), // cycles +1 if page crossed

    // Stores
    Opcode::new(0x85, "STA", 2, 3, AddressingMode::ZeroPage,  AccessType::Write, CPU::sta),
    Opcode::new(0x95, "STA", 2, 4, AddressingMode::ZeroPageX, AccessType::Write, CPU::sta),
    Opcode::new(0x8d, "STA", 3, 4, AddressingMode::Absolute,  AccessType::Write, CPU::sta),
    Opcode::new(0x9d, "STA", 3, 5, AddressingMode::AbsoluteX, AccessType::Write, CPU::sta),
    Opcode::new(0x99, "STA", 3, 5, AddressingMode::AbsoluteY, AccessType::Write, CPU::sta),
    Opcode::new(0x81, "STA", 2, 6, AddressingMode::IndirectX, AccessType::Write, CPU::sta),
    Opcode::new(0x91, "STA", 2, 6, AddressingMode::IndirectY, AccessType::Write, CPU::sta),

    Opcode::new(0x86, "STX", 2, 3, AddressingMode::ZeroPage,  AccessType::Read, CPU::stx),
    Opcode::new(0x96, "STX", 2, 4, AddressingMode::ZeroPageY, AccessType::Read, CPU::stx),
    Opcode::new(0x8e, "STX", 3, 4, AddressingMode::Absolute,  AccessType::Read, CPU::stx),

    Opcode::new(0x84, "STY", 2, 3, AddressingMode::ZeroPage,  AccessType::Read, CPU::sty),
    Opcode::new(0x94, "STY", 2, 4, AddressingMode::ZeroPageX, AccessType::Read, CPU::sty),
    Opcode::new(0x8c, "STY", 3, 4, AddressingMode::Absolute,  AccessType::Read, CPU::sty),

    // Stack
    Opcode::new(0x68, "PLA", 1, 4, AddressingMode::None, AccessType::Read,  CPU::pla),
    Opcode::new(0x28, "PLP", 1, 4, AddressingMode::None, AccessType::Read,  CPU::plp),
    Opcode::new(0x48, "PHA", 1, 3, AddressingMode::None, AccessType::Write, CPU::pha),
    Opcode::new(0x08, "PHP", 1, 3, AddressingMode::None, AccessType::Write, CPU::php),


    // Shifts
    Opcode::new(0x0A, "ASL", 1, 2, AddressingMode::Immediate, AccessType::Register,        CPU::asl_reg),
    Opcode::new(0x06, "ASL", 2, 5, AddressingMode::ZeroPage,  AccessType::ReadModifyWrite, CPU::asl_mem),
    Opcode::new(0x16, "ASL", 2, 6, AddressingMode::ZeroPageX, AccessType::ReadModifyWrite, CPU::asl_mem),
    Opcode::new(0x0E, "ASL", 3, 6, AddressingMode::Absolute,  AccessType::ReadModifyWrite, CPU::asl_mem),
    Opcode::new(0x1E, "ASL", 3, 7, AddressingMode::AbsoluteX, AccessType::ReadModifyWrite, CPU::asl_mem),

    Opcode::new(0x4A, "LSR", 1, 2, AddressingMode::Immediate, AccessType::Register,        CPU::lsr_reg),
    Opcode::new(0x46, "LSR", 2, 5, AddressingMode::ZeroPage,  AccessType::ReadModifyWrite, CPU::lsr_mem),
    Opcode::new(0x56, "LSR", 2, 6, AddressingMode::ZeroPageX, AccessType::ReadModifyWrite, CPU::lsr_mem),
    Opcode::new(0x4E, "LSR", 3, 6, AddressingMode::Absolute,  AccessType::ReadModifyWrite, CPU::lsr_mem),
    Opcode::new(0x5E, "LSR", 3, 7, AddressingMode::AbsoluteX, AccessType::ReadModifyWrite, CPU::lsr_mem),

    // Rotates
    Opcode::new(0x2A, "ROL", 1, 2, AddressingMode::Immediate, AccessType::Register,        CPU::rol_reg),
    Opcode::new(0x26, "ROL", 2, 5, AddressingMode::ZeroPage,  AccessType::ReadModifyWrite, CPU::rol_mem),
    Opcode::new(0x36, "ROL", 2, 6, AddressingMode::ZeroPageX, AccessType::ReadModifyWrite, CPU::rol_mem),
    Opcode::new(0x2E, "ROL", 3, 6, AddressingMode::Absolute,  AccessType::ReadModifyWrite, CPU::rol_mem),
    Opcode::new(0x3E, "ROL", 3, 7, AddressingMode::AbsoluteX, AccessType::ReadModifyWrite, CPU::rol_mem),

    Opcode::new(0x6A, "ROR", 1, 2, AddressingMode::Immediate, AccessType::Register,        CPU::ror_reg),
    Opcode::new(0x66, "ROR", 2, 5, AddressingMode::ZeroPage,  AccessType::ReadModifyWrite, CPU::ror_mem),
    Opcode::new(0x76, "ROR", 2, 6, AddressingMode::ZeroPageX, AccessType::ReadModifyWrite, CPU::ror_mem),
    Opcode::new(0x6E, "ROR", 3, 6, AddressingMode::Absolute,  AccessType::ReadModifyWrite, CPU::ror_mem),
    Opcode::new(0x7E, "ROR", 3, 7, AddressingMode::AbsoluteX, AccessType::ReadModifyWrite, CPU::ror_mem),

    // Increments
    Opcode::new(0xE6, "INC", 2, 5, AddressingMode::ZeroPage,  AccessType::ReadModifyWrite, CPU::inc),
    Opcode::new(0xF6, "INC", 2, 6, AddressingMode::ZeroPageX, AccessType::ReadModifyWrite, CPU::inc),
    Opcode::new(0xEE, "INC", 3, 6, AddressingMode::Absolute,  AccessType::ReadModifyWrite, CPU::inc),
    Opcode::new(0xFE, "INC", 3, 7, AddressingMode::AbsoluteX, AccessType::ReadModifyWrite, CPU::inc),

    Opcode::new(0xE8, "INX", 1, 2, AddressingMode::None, AccessType::Register, CPU::inx),
    Opcode::new(0xC8, "INY", 1, 2, AddressingMode::None, AccessType::Register, CPU::iny),

    // Decrements
    Opcode::new(0xC6, "DEC", 2, 5, AddressingMode::ZeroPage,  AccessType::ReadModifyWrite, CPU::dec),
    Opcode::new(0xD6, "DEC", 2, 6, AddressingMode::ZeroPageX, AccessType::ReadModifyWrite, CPU::dec),
    Opcode::new(0xCE, "DEC", 3, 6, AddressingMode::Absolute,  AccessType::ReadModifyWrite, CPU::dec),
    Opcode::new(0xDE, "DEC", 3, 7, AddressingMode::AbsoluteX, AccessType::ReadModifyWrite, CPU::dec),

    Opcode::new(0xCA, "DEX", 1, 2, AddressingMode::None, AccessType::Register, CPU::dex),
    Opcode::new(0x88, "DEY", 1, 2, AddressingMode::None, AccessType::Register, CPU::dey),

    // Comparisons
    Opcode::new(0xC9, "CMP", 2, 2, AddressingMode::Immediate, AccessType::Read, CPU::cmp),
    Opcode::new(0xC5, "CMP", 2, 3, AddressingMode::ZeroPage,  AccessType::Read, CPU::cmp),
    Opcode::new(0xD5, "CMP", 2, 4, AddressingMode::ZeroPageX, AccessType::Read, CPU::cmp),
    Opcode::new(0xCD, "CMP", 3, 4, AddressingMode::Absolute,  AccessType::Read, CPU::cmp),
    Opcode::new(0xDD, "CMP", 3, 4, AddressingMode::AbsoluteX, AccessType::Read, CPU::cmp), // cycles +1 if page crossed
    Opcode::new(0xD9, "CMP", 3, 4, AddressingMode::AbsoluteY, AccessType::Read, CPU::cmp), // cycles +1 if page crossed
    Opcode::new(0xC1, "CMP", 2, 6, AddressingMode::IndirectX, AccessType::Read, CPU::cmp),
    Opcode::new(0xD1, "CMP", 2, 5, AddressingMode::IndirectY, AccessType::Read, CPU::cmp), // cycles +1 if page crossed

    Opcode::new(0xE0, "CPX", 2, 2, AddressingMode::Immediate, AccessType::Read, CPU::cpx),
    Opcode::new(0xE4, "CPX", 2, 3, AddressingMode::ZeroPage,  AccessType::Read, CPU::cpx),
    Opcode::new(0xEC, "CPX", 3, 4, AddressingMode::Absolute,  AccessType::Read, CPU::cpx),

    Opcode::new(0xC0, "CPY", 2, 2, AddressingMode::Immediate, AccessType::Read, CPU::cpy),
    Opcode::new(0xC4, "CPY", 2, 3, AddressingMode::ZeroPage,  AccessType::Read, CPU::cpy),
    Opcode::new(0xCC, "CPY", 3, 4, AddressingMode::Absolute,  AccessType::Read, CPU::cpy),

    // Addition/Subtraction
    Opcode::new(0x69, "ADC", 2, 2, AddressingMode::Immediate, AccessType::Read, CPU::adc),
    Opcode::new(0x65, "ADC", 2, 3, AddressingMode::ZeroPage,  AccessType::Read, CPU::adc),
    Opcode::new(0x75, "ADC", 2, 4, AddressingMode::ZeroPageX, AccessType::Read, CPU::adc),
    Opcode::new(0x6D, "ADC", 3, 4, AddressingMode::Absolute,  AccessType::Read, CPU::adc),
    Opcode::new(0x7D, "ADC", 3, 4, AddressingMode::AbsoluteX, AccessType::Read, CPU::adc), // cycles +1 if page crossed
    Opcode::new(0x79, "ADC", 3, 4, AddressingMode::AbsoluteY, AccessType::Read, CPU::adc), // cycles +1 if page crossed
    Opcode::new(0x61, "ADC", 2, 6, AddressingMode::IndirectX, AccessType::Read, CPU::adc),
    Opcode::new(0x71, "ADC", 2, 5, AddressingMode::IndirectY, AccessType::Read, CPU::adc), // cycles +1 if page crossed

    Opcode::new(0xE9, "SBC", 2, 2, AddressingMode::Immediate, AccessType::Read, CPU::sbc),
    Opcode::new(0xE5, "SBC", 2, 3, AddressingMode::ZeroPage,  AccessType::Read, CPU::sbc),
    Opcode::new(0xF5, "SBC", 2, 4, AddressingMode::ZeroPageX, AccessType::Read, CPU::sbc),
    Opcode::new(0xED, "SBC", 3, 4, AddressingMode::Absolute,  AccessType::Read, CPU::sbc),
    Opcode::new(0xFD, "SBC", 3, 4, AddressingMode::AbsoluteX, AccessType::Read, CPU::sbc), // cycles +1 if page crossed
    Opcode::new(0xF9, "SBC", 3, 4, AddressingMode::AbsoluteY, AccessType::Read, CPU::sbc), // cycles +1 if page crossed
    Opcode::new(0xE1, "SBC", 2, 6, AddressingMode::IndirectX, AccessType::Read, CPU::sbc),
    Opcode::new(0xF1, "SBC", 2, 5, AddressingMode::IndirectY, AccessType::Read, CPU::sbc), // cycles +1 if page crossed

    // Bitwise Ops
    Opcode::new(0x29, "AND", 2, 2, AddressingMode::Immediate, AccessType::Read, CPU::and),
    Opcode::new(0x25, "AND", 2, 3, AddressingMode::ZeroPage,  AccessType::Read, CPU::and),
    Opcode::new(0x35, "AND", 2, 4, AddressingMode::ZeroPageX, AccessType::Read, CPU::and),
    Opcode::new(0x2D, "AND", 3, 4, AddressingMode::Absolute,  AccessType::Read, CPU::and),
    Opcode::new(0x3D, "AND", 3, 4, AddressingMode::AbsoluteX, AccessType::Read, CPU::and), // cycles +1 if page crossed
    Opcode::new(0x39, "AND", 3, 4, AddressingMode::AbsoluteY, AccessType::Read, CPU::and), // cycles +1 if page crossed
    Opcode::new(0x21, "AND", 2, 6, AddressingMode::IndirectX, AccessType::Read, CPU::and),
    Opcode::new(0x31, "AND", 2, 5, AddressingMode::IndirectY, AccessType::Read, CPU::and), // cycles +1 if page crossed

    Opcode::new(0x49, "EOR", 2, 2, AddressingMode::Immediate, AccessType::Read, CPU::eor),
    Opcode::new(0x45, "EOR", 2, 3, AddressingMode::ZeroPage,  AccessType::Read, CPU::eor),
    Opcode::new(0x55, "EOR", 2, 4, AddressingMode::ZeroPageX, AccessType::Read, CPU::eor),
    Opcode::new(0x4D, "EOR", 3, 4, AddressingMode::Absolute,  AccessType::Read, CPU::eor),
    Opcode::new(0x5D, "EOR", 3, 4, AddressingMode::AbsoluteX, AccessType::Read, CPU::eor), // cycles +1 if page crossed
    Opcode::new(0x59, "EOR", 3, 4, AddressingMode::AbsoluteY, AccessType::Read, CPU::eor), // cycles +1 if page crossed
    Opcode::new(0x41, "EOR", 2, 6, AddressingMode::IndirectX, AccessType::Read, CPU::eor),
    Opcode::new(0x51, "EOR", 2, 5, AddressingMode::IndirectY, AccessType::Read, CPU::eor), // cycles +1 if page crossed

    Opcode::new(0x09, "ORA", 2, 2, AddressingMode::Immediate, AccessType::Read, CPU::ora),
    Opcode::new(0x05, "ORA", 2, 3, AddressingMode::ZeroPage,  AccessType::Read, CPU::ora),
    Opcode::new(0x15, "ORA", 2, 4, AddressingMode::ZeroPageX, AccessType::Read, CPU::ora),
    Opcode::new(0x0D, "ORA", 3, 4, AddressingMode::Absolute,  AccessType::Read, CPU::ora),
    Opcode::new(0x1D, "ORA", 3, 4, AddressingMode::AbsoluteX, AccessType::Read, CPU::ora), // cycles +1 if page crossed
    Opcode::new(0x19, "ORA", 3, 4, AddressingMode::AbsoluteY, AccessType::Read, CPU::ora), // cycles +1 if page crossed
    Opcode::new(0x01, "ORA", 2, 6, AddressingMode::IndirectX, AccessType::Read, CPU::ora),
    Opcode::new(0x11, "ORA", 2, 5, AddressingMode::IndirectY, AccessType::Read, CPU::ora), // cycles +1 if page crossed

    // Jumps
    Opcode::new(0x4C, "JMP", 3, 3, AddressingMode::Absolute, AccessType::Read, CPU::jmp),
    Opcode::new(0x6C, "JMP", 3, 5, AddressingMode::Indirect, AccessType::Read, CPU::jmp), // Note: 6502 has a jmp bug here
    Opcode::new(0x20, "JSR", 3, 6, AddressingMode::Absolute, AccessType::Read, CPU::jsr),

    // Returns
    Opcode::new(0x60, "RTS", 1, 6, AddressingMode::None, AccessType::Read, CPU::rts),
    Opcode::new(0x40, "RTI", 1, 6, AddressingMode::None, AccessType::Read, CPU::rti),

    // Branches
    Opcode::new(0xD0, "BNE", 2, 2, AddressingMode::Relative, AccessType::Read, CPU::bne), // cycles +1 if branch succeeds, +2 if to a new page
    Opcode::new(0x70, "BVS", 2, 2, AddressingMode::Relative, AccessType::Read, CPU::bvs), // ..
    Opcode::new(0x50, "BVC", 2, 2, AddressingMode::Relative, AccessType::Read, CPU::bvc), // ..
    Opcode::new(0x30, "BMI", 2, 2, AddressingMode::Relative, AccessType::Read, CPU::bmi), // ..
    Opcode::new(0xF0, "BEQ", 2, 2, AddressingMode::Relative, AccessType::Read, CPU::beq), // ..
    Opcode::new(0xB0, "BCS", 2, 2, AddressingMode::Relative, AccessType::Read, CPU::bcs), // ..
    Opcode::new(0x90, "BCC", 2, 2, AddressingMode::Relative, AccessType::Read, CPU::bcc), // ..
    Opcode::new(0x10, "BPL", 2, 2, AddressingMode::Relative, AccessType::Read, CPU::bpl), // ..

    // Bit Test
    Opcode::new(0x24, "BIT", 2, 3, AddressingMode::ZeroPage, AccessType::Read, CPU::bit),
    Opcode::new(0x2C, "BIT", 3, 4, AddressingMode::Absolute, AccessType::Read, CPU::bit),

    //
    // /////////////////////////
    // // Unofficial Opcodes
    // /////////////////////////

    // Various multi-byte NOPs
    Opcode::new(0x80, "*NOP", 2, 2, AddressingMode::Immediate, AccessType::Read, CPU::fat_nop),
    Opcode::new(0x82, "*NOP", 2, 2, AddressingMode::Immediate, AccessType::Read, CPU::fat_nop),
    Opcode::new(0x89, "*NOP", 2, 2, AddressingMode::Immediate, AccessType::Read, CPU::fat_nop),
    Opcode::new(0xC2, "*NOP", 2, 2, AddressingMode::Immediate, AccessType::Read, CPU::fat_nop),
    Opcode::new(0xE2, "*NOP", 2, 2, AddressingMode::Immediate, AccessType::Read, CPU::fat_nop),
    Opcode::new(0x04, "*NOP", 2, 3, AddressingMode::ZeroPage,  AccessType::Read, CPU::fat_nop),
    Opcode::new(0x44, "*NOP", 2, 3, AddressingMode::ZeroPage,  AccessType::Read, CPU::fat_nop),
    Opcode::new(0x64, "*NOP", 2, 3, AddressingMode::ZeroPage,  AccessType::Read, CPU::fat_nop),
    Opcode::new(0x14, "*NOP", 2, 4, AddressingMode::ZeroPageX, AccessType::Read, CPU::fat_nop),
    Opcode::new(0x34, "*NOP", 2, 4, AddressingMode::ZeroPageX, AccessType::Read, CPU::fat_nop),
    Opcode::new(0x54, "*NOP", 2, 4, AddressingMode::ZeroPageX, AccessType::Read, CPU::fat_nop),
    Opcode::new(0x74, "*NOP", 2, 4, AddressingMode::ZeroPageX, AccessType::Read, CPU::fat_nop),
    Opcode::new(0xD4, "*NOP", 2, 4, AddressingMode::ZeroPageX, AccessType::Read, CPU::fat_nop),
    Opcode::new(0xF4, "*NOP", 2, 4, AddressingMode::ZeroPageX, AccessType::Read, CPU::fat_nop),
    Opcode::new(0x0C, "*NOP", 3, 4, AddressingMode::Absolute,  AccessType::Read, CPU::fat_nop),
    Opcode::new(0x1C, "*NOP", 3, 4, AddressingMode::AbsoluteX, AccessType::Read, CPU::fat_nop), // cycles +1 if page crossed
    Opcode::new(0x3C, "*NOP", 3, 4, AddressingMode::AbsoluteX, AccessType::Read, CPU::fat_nop), // cycles +1 if page crossed
    Opcode::new(0x5C, "*NOP", 3, 4, AddressingMode::AbsoluteX, AccessType::Read, CPU::fat_nop), // cycles +1 if page crossed
    Opcode::new(0x7C, "*NOP", 3, 4, AddressingMode::AbsoluteX, AccessType::Read, CPU::fat_nop), // cycles +1 if page crossed
    Opcode::new(0xDC, "*NOP", 3, 4, AddressingMode::AbsoluteX, AccessType::Read, CPU::fat_nop), // cycles +1 if page crossed
    Opcode::new(0xFC, "*NOP", 3, 4, AddressingMode::AbsoluteX, AccessType::Read, CPU::fat_nop), // cycles +1 if page crossed

    // Various slow NOPs
    Opcode::new(0x12, "*NOP", 1, 11, AddressingMode::None, AccessType::Read, CPU::nop),
    Opcode::new(0x22, "*NOP", 1, 11, AddressingMode::None, AccessType::Read, CPU::nop),
    Opcode::new(0x32, "*NOP", 1, 11, AddressingMode::None, AccessType::Read, CPU::nop),
    Opcode::new(0x42, "*NOP", 1, 11, AddressingMode::None, AccessType::Read, CPU::nop),
    Opcode::new(0x52, "*NOP", 1, 11, AddressingMode::None, AccessType::Read, CPU::nop),
    Opcode::new(0x62, "*NOP", 1, 11, AddressingMode::None, AccessType::Read, CPU::nop),
    Opcode::new(0x72, "*NOP", 1, 11, AddressingMode::None, AccessType::Read, CPU::nop),
    Opcode::new(0x92, "*NOP", 1, 11, AddressingMode::None, AccessType::Read, CPU::nop),
    Opcode::new(0xB2, "*NOP", 1, 11, AddressingMode::None, AccessType::Read, CPU::nop),
    Opcode::new(0xD2, "*NOP", 1, 11, AddressingMode::None, AccessType::Read, CPU::nop),
    Opcode::new(0xF2, "*NOP", 1, 11, AddressingMode::None, AccessType::Read, CPU::nop),

    // Various well-behaved NOPs
    Opcode::new(0x1A, "*NOP", 1, 2,  AddressingMode::None, AccessType::Read, CPU::nop),
    Opcode::new(0x3A, "*NOP", 1, 2,  AddressingMode::None, AccessType::Read, CPU::nop),
    Opcode::new(0x5A, "*NOP", 1, 2,  AddressingMode::None, AccessType::Read, CPU::nop),
    Opcode::new(0x7A, "*NOP", 1, 2,  AddressingMode::None, AccessType::Read, CPU::nop),
    Opcode::new(0xDA, "*NOP", 1, 2,  AddressingMode::None, AccessType::Read, CPU::nop),
    Opcode::new(0xFA, "*NOP", 1, 2,  AddressingMode::None, AccessType::Read, CPU::nop),

    // DCP => DEC oper + CMP oper
    Opcode::new(0xC7, "*DCP", 2, 5, AddressingMode::ZeroPage,  AccessType::ReadModifyWrite, CPU::dcp),
    Opcode::new(0xD7, "*DCP", 2, 6, AddressingMode::ZeroPageX, AccessType::ReadModifyWrite, CPU::dcp),
    Opcode::new(0xCF, "*DCP", 3, 6, AddressingMode::Absolute,  AccessType::ReadModifyWrite, CPU::dcp),
    Opcode::new(0xDF, "*DCP", 3, 7, AddressingMode::AbsoluteX, AccessType::ReadModifyWrite, CPU::dcp),
    Opcode::new(0xDB, "*DCP", 3, 7, AddressingMode::AbsoluteY, AccessType::ReadModifyWrite, CPU::dcp),
    Opcode::new(0xD3, "*DCP", 2, 8, AddressingMode::IndirectY, AccessType::ReadModifyWrite, CPU::dcp),
    Opcode::new(0xC3, "*DCP", 2, 8, AddressingMode::IndirectX, AccessType::ReadModifyWrite, CPU::dcp),

    // RLA => ROL oper + AND oper
    Opcode::new(0x27, "*RLA", 2, 5, AddressingMode::ZeroPage,  AccessType::ReadModifyWrite, CPU::rla),
    Opcode::new(0x37, "*RLA", 2, 6, AddressingMode::ZeroPageX, AccessType::ReadModifyWrite, CPU::rla),
    Opcode::new(0x2F, "*RLA", 3, 6, AddressingMode::Absolute,  AccessType::ReadModifyWrite, CPU::rla),
    Opcode::new(0x3F, "*RLA", 3, 7, AddressingMode::AbsoluteX, AccessType::ReadModifyWrite, CPU::rla),
    Opcode::new(0x3B, "*RLA", 3, 7, AddressingMode::AbsoluteY, AccessType::ReadModifyWrite, CPU::rla),
    Opcode::new(0x33, "*RLA", 2, 8, AddressingMode::IndirectY, AccessType::ReadModifyWrite, CPU::rla),
    Opcode::new(0x23, "*RLA", 2, 8, AddressingMode::IndirectX, AccessType::ReadModifyWrite, CPU::rla),

    // SLO => ASL oper + ORA oper
    Opcode::new(0x07, "*SLO", 2, 5, AddressingMode::ZeroPage,  AccessType::ReadModifyWrite, CPU::slo),
    Opcode::new(0x17, "*SLO", 2, 6, AddressingMode::ZeroPageX, AccessType::ReadModifyWrite, CPU::slo),
    Opcode::new(0x0F, "*SLO", 3, 6, AddressingMode::Absolute,  AccessType::ReadModifyWrite, CPU::slo),
    Opcode::new(0x1F, "*SLO", 3, 7, AddressingMode::AbsoluteX, AccessType::ReadModifyWrite, CPU::slo),
    Opcode::new(0x1B, "*SLO", 3, 7, AddressingMode::AbsoluteY, AccessType::ReadModifyWrite, CPU::slo),
    Opcode::new(0x03, "*SLO", 2, 8, AddressingMode::IndirectX, AccessType::ReadModifyWrite, CPU::slo),
    Opcode::new(0x13, "*SLO", 2, 8, AddressingMode::IndirectY, AccessType::ReadModifyWrite, CPU::slo),

    // SRE => LSR oper + EOR oper
    Opcode::new(0x47, "*SRE", 2, 5, AddressingMode::ZeroPage,  AccessType::ReadModifyWrite, CPU::sre),
    Opcode::new(0x57, "*SRE", 2, 6, AddressingMode::ZeroPageX, AccessType::ReadModifyWrite, CPU::sre),
    Opcode::new(0x4F, "*SRE", 3, 6, AddressingMode::Absolute,  AccessType::ReadModifyWrite, CPU::sre),
    Opcode::new(0x5F, "*SRE", 3, 7, AddressingMode::AbsoluteX, AccessType::ReadModifyWrite, CPU::sre),
    Opcode::new(0x5B, "*SRE", 3, 7, AddressingMode::AbsoluteY, AccessType::ReadModifyWrite, CPU::sre),
    Opcode::new(0x43, "*SRE", 2, 8, AddressingMode::IndirectX, AccessType::ReadModifyWrite, CPU::sre),
    Opcode::new(0x53, "*SRE", 2, 8, AddressingMode::IndirectY, AccessType::ReadModifyWrite, CPU::sre),

    // RRA => ROR oper + ADC oper
    Opcode::new(0x67, "*RRA", 2, 5, AddressingMode::ZeroPage,  AccessType::ReadModifyWrite, CPU::rra),
    Opcode::new(0x77, "*RRA", 2, 6, AddressingMode::ZeroPageX, AccessType::ReadModifyWrite, CPU::rra),
    Opcode::new(0x6F, "*RRA", 3, 6, AddressingMode::Absolute,  AccessType::ReadModifyWrite, CPU::rra),
    Opcode::new(0x7F, "*RRA", 3, 7, AddressingMode::AbsoluteX, AccessType::ReadModifyWrite, CPU::rra),
    Opcode::new(0x7B, "*RRA", 3, 7, AddressingMode::AbsoluteY, AccessType::ReadModifyWrite, CPU::rra),
    Opcode::new(0x63, "*RRA", 2, 8, AddressingMode::IndirectX, AccessType::ReadModifyWrite, CPU::rra),
    Opcode::new(0x73, "*RRA", 2, 8, AddressingMode::IndirectY, AccessType::ReadModifyWrite, CPU::rra),

    // ISC => INC oper + SBC oper
    Opcode::new(0xE7, "*ISC", 2,5, AddressingMode::ZeroPage,  AccessType::ReadModifyWrite, CPU::isc),
    Opcode::new(0xF7, "*ISC", 2,6, AddressingMode::ZeroPageX, AccessType::ReadModifyWrite, CPU::isc),
    Opcode::new(0xEF, "*ISC", 3,6, AddressingMode::Absolute,  AccessType::ReadModifyWrite, CPU::isc),
    Opcode::new(0xFF, "*ISC", 3,7, AddressingMode::AbsoluteX, AccessType::ReadModifyWrite, CPU::isc),
    Opcode::new(0xFB, "*ISC", 3,7, AddressingMode::AbsoluteY, AccessType::ReadModifyWrite, CPU::isc),
    Opcode::new(0xE3, "*ISC", 2,8, AddressingMode::IndirectX, AccessType::ReadModifyWrite, CPU::isc),
    Opcode::new(0xF3, "*ISC", 2,8, AddressingMode::IndirectY, AccessType::ReadModifyWrite, CPU::isc),

    // LAX => LDA oper + LDX oper
    Opcode::new(0xA7, "*LAX", 2, 3, AddressingMode::ZeroPage,  AccessType::Read, CPU::lax),
    Opcode::new(0xB7, "*LAX", 2, 4, AddressingMode::ZeroPageY, AccessType::Read, CPU::lax),
    Opcode::new(0xAF, "*LAX", 3, 4, AddressingMode::Absolute,  AccessType::Read, CPU::lax),
    Opcode::new(0xBF, "*LAX", 3, 4, AddressingMode::AbsoluteY, AccessType::Read, CPU::lax),
    Opcode::new(0xA3, "*LAX", 2, 6, AddressingMode::IndirectX, AccessType::Read, CPU::lax),
    Opcode::new(0xB3, "*LAX", 2, 5, AddressingMode::IndirectY, AccessType::Read, CPU::lax),

    // SAX => A AND X -> M
    Opcode::new(0x87, "*SAX", 2, 3, AddressingMode::ZeroPage,  AccessType::Write, CPU::sax),
    Opcode::new(0x97, "*SAX", 2, 4, AddressingMode::ZeroPageY, AccessType::Write, CPU::sax),
    Opcode::new(0x8F, "*SAX", 3, 4, AddressingMode::Absolute,  AccessType::Write, CPU::sax),
    Opcode::new(0x83, "*SAX", 2, 6, AddressingMode::IndirectX, AccessType::Write, CPU::sax),

    // SBX (AXS, SAX) => CMP and DEX at once, sets flags like CMP
    Opcode::new(0xCB, "*SBX", 2,2, AddressingMode::Immediate, AccessType::Read, CPU::sbx),

    // ARR => AND oper + ROR (Plus some wonky flag manipulation)
    Opcode::new(0x6B, "*ARR", 2,2, AddressingMode::Immediate, AccessType::Read, CPU::arr),

    // USBC (SBC) => SBC oper + NOP
    Opcode::new(0xEB, "*USBC", 2,2, AddressingMode::Immediate, AccessType::Read, CPU::usbc),

    // ANC => A AND oper, bit(7) -> C
    Opcode::new(0x0B, "*ANC", 2,2, AddressingMode::Immediate, AccessType::Read, CPU::anc),
    Opcode::new(0x2B, "*ANC", 2,2, AddressingMode::Immediate, AccessType::Read, CPU::anc),

    // ALR => AND oper + LSR
    Opcode::new(0x4B, "*ALR", 2,2, AddressingMode::Immediate, AccessType::Read, CPU::alr),

    // LAS (LAR) => AND with SP, store in A, X, SP
    Opcode::new(0xBB, "*LAS", 3,4, AddressingMode::AbsoluteY, AccessType::Read, CPU::las), // cycles +1 if page page crossed

    // Kill for debugging (technically this entry is never accessed)
    Opcode::new(0x02, "*KIL/JAM", 1,11, AddressingMode::None, AccessType::None, CPU::jam),

    // Too Unstable to implement
    Opcode::new(0x8B, "*UNSTABLE", 1,1, AddressingMode::None, AccessType::Read, CPU::unstable),
    Opcode::new(0xAB, "*UNSTABLE", 1,1, AddressingMode::None, AccessType::Read, CPU::unstable),
    Opcode::new(0x9F, "*UNSTABLE", 1,1, AddressingMode::None, AccessType::Read, CPU::unstable),
    Opcode::new(0x93, "*UNSTABLE", 1,1, AddressingMode::None, AccessType::Read, CPU::unstable),
    Opcode::new(0x9E, "*UNSTABLE", 1,1, AddressingMode::None, AccessType::Read, CPU::unstable),
    Opcode::new(0x9C, "*UNSTABLE", 1,1, AddressingMode::None, AccessType::Read, CPU::unstable),
    Opcode::new(0x9B, "*UNSTABLE", 1,1, AddressingMode::None, AccessType::Read, CPU::unstable),

    // Jams


];

pub static OPCODES_MAP: Lazy<HashMap<u8, &Opcode>> = Lazy::new(|| {
    let mut map = HashMap::new();
    for opcode in OPCODES {
        map.insert(opcode.code, opcode);
    }
    map
});
