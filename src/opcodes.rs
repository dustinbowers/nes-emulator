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
    Opcode::new(0, "BRK", 1, 1, AddressingMode::None),

    Opcode::new(0xA9, "LDA", 2, 2, AddressingMode::Immediate),
    Opcode::new(0xA5, "LDA", 2, 3, AddressingMode::ZeroPage),
    Opcode::new(0xB5, "LDA", 2, 4, AddressingMode::ZeroPageX),
    Opcode::new(0xAD, "LDA", 3, 4, AddressingMode::Absolute),
    Opcode::new(0xBD, "LDA", 3, 4, AddressingMode::AbsoluteX), // cycles +1 if page crossed
    Opcode::new(0xB9, "LDA", 3, 4, AddressingMode::AbsoluteY), // cycles +1 if page crossed
    Opcode::new(0xA1, "LDA", 2, 6, AddressingMode::IndirectX),
    Opcode::new(0xB1, "LDA", 2, 5, AddressingMode::IndirectY), // cycles + 1 if page crossed

    Opcode::new(0xAA, "TAX", 1, 2, AddressingMode::None),
];

pub static OPCODES_MAP: Lazy<HashMap<u8, &Opcode>> = Lazy::new(|| {
    let mut map = HashMap::new();
    for opcode in OPCODES {
        map.insert(opcode.value, opcode);
    }
    map
});
