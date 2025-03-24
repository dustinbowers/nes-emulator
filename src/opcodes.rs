use crate::cpu::AddressingMode;
use once_cell::sync::Lazy;
use std::collections::HashMap;

pub struct Opcode {
    pub value: u8,
    pub name: &'static str,
    pub cycles: u8,
    pub size: u8,
    pub mode: AddressingMode,
}

impl Opcode {
    pub const fn new(value: u8, name: &'static str, size: u8, cycles: u8, mode: AddressingMode) -> Self {
        Self {
            value,
            name,
            cycles,
            size,
            mode,
        }
    }
}

#[rustfmt::skip]
const OPCODES: &[Opcode] = &[
    Opcode::new(0x00, "BRK", 1, 7, AddressingMode::None),
    Opcode::new(0xEA, "NOP", 1, 2, AddressingMode::None),

    // Loads
    Opcode::new(0xA9, "LDA", 2, 2, AddressingMode::Immediate),
    Opcode::new(0xA5, "LDA", 2, 3, AddressingMode::ZeroPage),
    Opcode::new(0xB5, "LDA", 2, 4, AddressingMode::ZeroPageX),
    Opcode::new(0xAD, "LDA", 3, 4, AddressingMode::Absolute),
    Opcode::new(0xBD, "LDA", 3, 4, AddressingMode::AbsoluteX), // cycles +1 if page crossed
    Opcode::new(0xB9, "LDA", 3, 4, AddressingMode::AbsoluteY), // cycles +1 if page crossed
    Opcode::new(0xA1, "LDA", 2, 6, AddressingMode::IndirectX),
    Opcode::new(0xB1, "LDA", 2, 5, AddressingMode::IndirectY), // cycles +1 if page crossed

    Opcode::new(0xA2, "LDX", 2, 2, AddressingMode::Immediate),
    Opcode::new(0xA6, "LDX", 2, 3, AddressingMode::ZeroPage),
    Opcode::new(0xB6, "LDX", 2, 4, AddressingMode::ZeroPageY),
    Opcode::new(0xAE, "LDX", 3, 4, AddressingMode::Absolute),
    Opcode::new(0xBE, "LDX", 3, 4, AddressingMode::AbsoluteY), // cycles +1 if page crossed

    Opcode::new(0xA0, "LDY", 2, 2, AddressingMode::Immediate),
    Opcode::new(0xA4, "LDY", 2, 3, AddressingMode::ZeroPage),
    Opcode::new(0xB4, "LDY", 2, 4, AddressingMode::ZeroPageX),
    Opcode::new(0xAC, "LDY", 3, 4, AddressingMode::Absolute),
    Opcode::new(0xBC, "LDY", 3, 4, AddressingMode::AbsoluteX), // cycles +1 if page crossed

    // Stores
    Opcode::new(0x85, "STA", 2, 3, AddressingMode::ZeroPage),
    Opcode::new(0x95, "STA", 2, 4, AddressingMode::ZeroPageX),
    Opcode::new(0x8d, "STA", 3, 4, AddressingMode::Absolute),
    Opcode::new(0x9d, "STA", 3, 5, AddressingMode::AbsoluteX),
    Opcode::new(0x99, "STA", 3, 5, AddressingMode::AbsoluteY),
    Opcode::new(0x81, "STA", 2, 6, AddressingMode::IndirectX),
    Opcode::new(0x91, "STA", 2, 6, AddressingMode::IndirectY),

    Opcode::new(0x86, "STX", 2, 3, AddressingMode::ZeroPage),
    Opcode::new(0x96, "STX", 2, 4, AddressingMode::ZeroPageY),
    Opcode::new(0x8e, "STX", 3, 4, AddressingMode::Absolute),

    Opcode::new(0x84, "STY", 2, 3, AddressingMode::ZeroPage),
    Opcode::new(0x94, "STY", 2, 4, AddressingMode::ZeroPageX),
    Opcode::new(0x8c, "STY", 3, 4, AddressingMode::Absolute),

    Opcode::new(0xAA, "TAX", 1, 2, AddressingMode::None),

    Opcode::new(0xE8, "INX", 1, 2, AddressingMode::None),
    Opcode::new(0xC8, "INY", 1, 2, AddressingMode::None),

    Opcode::new(0xca, "DEX", 1, 2, AddressingMode::None),
    Opcode::new(0x88, "DEY", 1, 2, AddressingMode::None),

    // Stack
    Opcode::new(0x48, "PHA", 1, 3, AddressingMode::None),
    Opcode::new(0x68, "PLA", 1, 4, AddressingMode::None),
    Opcode::new(0x08, "PHP", 1, 3, AddressingMode::None),
    Opcode::new(0x28, "PLP", 1, 4, AddressingMode::None),

];

pub static OPCODES_MAP: Lazy<HashMap<u8, &Opcode>> = Lazy::new(|| {
    let mut map = HashMap::new();
    for opcode in OPCODES {
        map.insert(opcode.value, opcode);
    }
    map
});
