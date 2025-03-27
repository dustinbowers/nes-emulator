use crate::{opcodes, Bus};
use bitflags::bitflags;
use std::collections::HashMap;
use std::future::Future;

const DEBUG: bool = false;
const CPU_PC_RESET: u16 = 0x8000;
const CPU_STACK_RESET: u8 = 0xFF;
const CPU_STACK_BASE: u16 = 0x0100;

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

#[derive(Debug)]
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
    Indirect, // Only JMP supports this mode
    Relative, // The branch instructions exclusively use this mode
    None,
}

pub struct CPU {
    bus: Bus,

    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub stack_pointer: u8,
    pub status: Flags,
    pub program_counter: u16,

    extra_cycles: u8,
    skip_pc_advance: bool,
}

impl CPU {
    pub fn new(bus: Bus) -> CPU {
        CPU {
            bus,
            register_a: 0,
            register_x: 0,
            register_y: 0,
            stack_pointer: CPU_STACK_RESET,
            status: Flags::from_bits_truncate(0b0010_0010),
            program_counter: CPU_PC_RESET,
            extra_cycles: 0,
            skip_pc_advance: false,
        }
    }

    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.stack_pointer = CPU_STACK_RESET;
        self.program_counter = CPU_PC_RESET;
        self.status = Flags::from_bits_truncate(0b0010_0010);
        self.extra_cycles = 0;
        self.skip_pc_advance = false;
    }

    pub fn load(&mut self, program: &[u8]) {
        self.reset();
        self.load_program_at(program, self.program_counter);
    }

    pub fn load_program_at(&mut self, program: &[u8], address: u16) {
        self.bus.store_bytes(address, program);
    }

    pub fn fetch_byte(&mut self, address: u16) -> u8 {
        self.bus.fetch_byte(address)
    }

    pub fn fetch_bytes_raw(&mut self, address: u16, size: u16) -> &[u8] {
        self.bus.fetch_bytes_raw(address, size)
    }

    pub fn fetch_u16(&mut self, address: u16) -> u16 {
        let lo = self.bus.fetch_byte(address) as u16;
        let hi = self.bus.fetch_byte(address.wrapping_add(1)) as u16;
        hi << 8 | lo
    }

    pub fn store_byte(&mut self, address: u16, value: u8) {
        self.bus.store_byte(address, value);
    }

    pub async fn run(&mut self) {
        self.run_with_callback(|_| {}).await;
    }

    pub async fn run_with_callback<F>(&mut self, mut callback: F)
    where
        F: FnMut(&mut CPU),
    {
        loop {
            callback(self);
            let (cycles, bytes_consumed, should_break) = self.tick();
            if should_break {
                break;
            }
        }
    }

    // `tick` returns (num_cycles, bytes_consumed, is_breaking)
    pub fn tick(&mut self) -> (u8, u8, bool) {
        let ref opcodes: HashMap<u8, &'static opcodes::Opcode> = *opcodes::OPCODES_MAP;

        self.extra_cycles = 0;
        self.skip_pc_advance = false;
        let code = self.fetch_byte(self.program_counter);
        let opcode = *opcodes
            .get(&code)
            .expect(&format!("Unknown opcode: {:#x}", &code));

        if DEBUG {
            let mut operand_bytes: Vec<u8> = vec![];
            for i in 1..opcode.size {
                let address = self.program_counter.wrapping_add(i as u16);
                operand_bytes.push(self.fetch_byte(address));
            }
            println!(
                "({}) PC:${:04X} SP:${:02X} A:${:02X} X:${:02X} Y:${:02X} status: 0b{:08b} \tOpcode: (${:02X}) {} {:02X?}",
                self.program_counter,
                self.program_counter,
                self.stack_pointer,
                self.register_a,
                self.register_x,
                self.register_y,
                self.status.bits(),
                self.bus.fetch_byte(self.program_counter),
                opcode.name,
                operand_bytes
            )
        }
        self.program_counter = self.program_counter.wrapping_add(1);

        match code {
            0x00 => return (1, 1, true), // BRK
            0xEA => {}                   // NOP

            0x4C => self.jmp(opcode), // JMP Absolute
            0x6C => self.jmp(opcode), // JMP Indirect (with 6502 bug)
            0x20 => self.jsr(opcode), // JSR
            0x60 => self.rts(),       // RTS
            0x40 => self.rti(),       // RTI

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

            0xD0 => self.bne(opcode), // BNE
            0x70 => self.bvs(opcode), // BVS
            0x50 => self.bvc(opcode), // BVC
            0x30 => self.bmi(opcode), // BMI
            0xF0 => self.beq(opcode), // BEQ
            0xB0 => self.bcs(opcode), // BCS
            0x90 => self.bcc(opcode), // BCC
            0x10 => self.bpl(opcode), // BPL

            0xE8 => self.inx(), // INX
            0xC8 => self.iny(), // INY

            0xCA => self.dex(), // DEX
            0x88 => self.dey(), // DEY

            0x48 => self.pha(), // PHA
            0x68 => self.pla(), // PLA
            0x08 => self.php(), // PHP
            0x28 => self.plp(), // PLP

            0x24 => self.bit(opcode), // BIT

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
            0x6A | 0x66 | 0x76 | 0x6E | 0x7E => {
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
            0x09 | 0x05 | 0x15 | 0x0D | 0x1D | 0x19 | 0x01 | 0x11 => {
                self.ora(opcode); // ORA
            }

            /////////////////////////
            /// Unofficial Opcodes
            /////////////////////////
            0x80 | 0x82 | 0x89 | 0xC2 | 0xE2 | 0x04 | 0x44 | 0x64 | 0x14 | 0x34 | 0x54 | 0x74
            | 0xD4 | 0xF4 | 0x0C | 0x1C | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC | 0x02 | 0x12 | 0x22
            | 0x32 | 0x42 | 0x52 | 0x62 | 0x72 | 0x92 | 0xB2 | 0xD2 | 0xF2 | 0x1A | 0x3A | 0x5A
            | 0x7A | 0xDA | 0xFA => {
                // Various single and multiple-byte NOPs
            }

            0xC7 | 0xD7 | 0xCF | 0xDF | 0xDB | 0xD3 | 0xC3 => {
                // DCP => DEC oper + CMP oper
                self.dec(opcode);
                self.cmp(opcode);
            }
            0x27 | 0x37 | 0x2F | 0x3F | 0x3B | 0x33 | 0x23 => {
                // RLA => ROL oper + AND oper
                self.rol(opcode);
                self.and(opcode);
            }
            0x07 | 0x17 | 0x0F | 0x1F | 0x1B | 0x03 | 0x13 => {
                // SLO => ASL oper + ORA oper
                self.asl(opcode);
                self.ora(opcode);
            }
            0x47 | 0x57 | 0x4F | 0x5F | 0x5B | 0x43 | 0x53 => {
                // SRE => LSR oper + EOR oper
                self.lsr(opcode);
                self.eor(opcode);
            }
            0x67 | 0x77 | 0x6F | 0x7F | 0x7B | 0x63 | 0x73 => {
                // RRA => ROR oper + ADC oper
                self.ror(opcode);
                self.adc(opcode);
            }
            0xE7 | 0xF7 | 0xEF | 0xFF | 0xFB | 0xE3 | 0xF3 => {
                // ISC (ISB / INS) => INC oper + SBC oper
                self.inc(opcode);
                self.sbc(opcode);
            }
            0xA7 | 0xB7 | 0xAF | 0xBF | 0xA3 | 0xB3 => {
                // LAX => LDA oper + LDX oper
                self.lda(opcode);
                self.ldx(opcode);
            }
            0x87 | 0x97 | 0x8F | 0x83 => {
                // SAX => A AND X -> M
                self.sax(opcode);
            }
            0xCB => {
                // SBX => CMP and DEX at once, sets flags like CMP
                self.sbx(opcode);
            }
            0x6B => {
                todo!();
            }
            0xEB => {
                // USBC (SBC) => SBC oper + NOP
                self.sbc(opcode);
                // NOP
            }
            0x0B => {
                // ANC => A AND oper, bit(7) -> C
                self.anc(opcode);
            }
            0x4B => {
                // ALR => AND oper + LSR
                self.and(opcode);
                self.lsr(opcode);
            }

            _ => todo!(),
        }

        // Tick the bus for opcode cycles. Add any extra cycles from boundary_crosses and other special cases
        let cycle_count = opcode.cycles + self.extra_cycles;
        self.bus.tick(cycle_count as usize);

        // If the opcode didn't move PC by some call/ret/branch, then
        // we step it forward by the size of the opcode - 1
        // since we've already stepped it forward one byte when reading it
        if !self.skip_pc_advance {
            self.program_counter = self.program_counter.wrapping_add((opcode.size - 1) as u16);
        }
        (cycle_count, opcode.size, false)
    }

    // Utility functions
    /////////////////////
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
            AddressingMode::Indirect => {
                // Note: JMP is the only opcode to use this AddressingMode
                /* NOTE:
                  An original 6502 has does not correctly fetch the target address if the indirect vector falls
                  on a page boundary (e.g. $xxFF where xx is any value from $00 to $FF). In this case fetches
                  the LSB from $xxFF as expected but takes the MSB from $xx00.
                */
                let indirect_vec = self.fetch_u16(self.program_counter);
                let address = if indirect_vec & 0x00FF == 0x00FF {
                    let lo = self.fetch_byte(indirect_vec) as u16;
                    let hi = self.fetch_byte(indirect_vec & 0xFF00) as u16;
                    (hi << 2) | lo
                } else {
                    indirect_vec
                };
                (address, false)
            }
            AddressingMode::Relative => {
                // Note: Branch opcodes exclusively use this address mode
                let offset = self.fetch_byte(self.program_counter) as i8; // sign-extend u8 to i8
                let base_pc = self.program_counter.wrapping_add(1); // the relative address is based on a PC /after/ the current opcode
                let target_address = base_pc.wrapping_add_signed(offset as i16);
                let boundary_crossed = is_boundary_crossed(base_pc, target_address); // TODO: this might not be right...
                (target_address, boundary_crossed)
            }
            _ => unimplemented!(),
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

    fn set_register_y(&mut self, value: u8) {
        self.register_y = value;
        self.update_zero_and_negative_flags(value);
    }

    fn set_program_counter(&mut self, address: u16) {
        self.program_counter = address;
        self.skip_pc_advance = true;
    }

    fn stack_push(&mut self, value: u8) {
        let address = CPU_STACK_BASE.wrapping_add(self.stack_pointer as u16);
        self.bus.store_byte(address, value);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
    }

    fn stack_push_u16(&mut self, value: u16) {
        let hi = (value >> 8) as u8;
        let lo = value as u8;
        self.stack_push(hi);
        self.stack_push(lo);
    }

    fn stack_pop(&mut self) -> u8 {
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        self.bus
            .fetch_byte(CPU_STACK_BASE.wrapping_add(self.stack_pointer as u16))
    }

    fn stack_pop_u16(&mut self) -> u16 {
        let lo = self.stack_pop() as u16;
        let hi = self.stack_pop() as u16;
        hi << 8 | lo
    }

    fn update_zero_and_negative_flags(&mut self, result: u8) {
        self.status.set(Flags::ZERO, result == 0);
        self.status.set(Flags::NEGATIVE, result & 0b1000_0000 != 0);
    }

    fn add_to_register_a(&mut self, value: u8) {
        let curr_carry = self.status.contains(Flags::CARRY) as u8;
        let sum = self.register_a as u16 + value as u16 + curr_carry as u16;
        let result = sum as u8;

        // Method: OVERFLOW if the sign of the inputs are the same,
        //         and do not match the sign of the result
        // Reasoning: A signed overflow MUST have occurred in these cases:
        //              * Positive + Positive = Negative OR
        //              * Negative + Negative = Positive
        // Boolean logic: (!((register_a ^ value) & 0x80) && ((register_a ^ result) & 0x80))
        // See: https://forums.nesdev.org/viewtopic.php?t=6331
        let signed_overflow =
            ((self.register_a ^ result) & 0x80 != 0) && ((self.register_a ^ value) & 0x80 == 0);

        self.status.set(Flags::OVERFLOW, signed_overflow);
        self.status.set(Flags::NEGATIVE, result & 0x80 != 0);
        self.status.set(Flags::ZERO, result == 0);
        self.status.set(Flags::CARRY, sum > 0xFF);
        self.register_a = result;
    }

    fn sub_from_register_a(&mut self, data: u8) {
        self.add_to_register_a(!data);
    }

    fn compare(&mut self, opcode: &opcodes::Opcode, compare_value: u8) {
        let (address, boundary_crossed) = self.get_parameter_address(&opcode.mode);
        let value = self.bus.fetch_byte(address);
        self.status.set(Flags::CARRY, compare_value >= value);
        self.update_zero_and_negative_flags(compare_value.wrapping_sub(value));
        self.extra_cycles += boundary_crossed as u8;
    }

    fn branch(&mut self, opcode: &opcodes::Opcode, condition: bool) {
        let (address, boundary_crossed) = self.get_parameter_address(&opcode.mode);
        let mut cycles = boundary_crossed as u8;
        if condition {
            self.set_program_counter(address);
            cycles += 1;
        }
        self.extra_cycles += cycles;
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
        let (address, _) = self.get_parameter_address(&opcode.mode);
        self.bus.store_byte(address, self.register_a);
    }

    fn stx(&mut self, opcode: &opcodes::Opcode) {
        let (address, _) = self.get_parameter_address(&opcode.mode);
        self.bus.store_byte(address, self.register_x);
    }

    fn sty(&mut self, opcode: &opcodes::Opcode) {
        let (address, _) = self.get_parameter_address(&opcode.mode);
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
        match opcode.mode {
            AddressingMode::Immediate => {
                let carry = self.register_a & 0x80 != 0;
                let value = self.register_a << 1;
                self.set_register_a(value);
                self.status.set(Flags::CARRY, carry);
            }
            _ => {
                let (address, _) = self.get_parameter_address(&opcode.mode);
                let mut value = self.fetch_byte(address);
                let carry = value & 0x80 != 0;
                value <<= 1;
                self.store_byte(address, value);
                self.update_zero_and_negative_flags(value);
                self.status.set(Flags::CARRY, carry);
            }
        }
    }

    fn lsr(&mut self, opcode: &opcodes::Opcode) {
        // Logical Shift Right into carry
        match opcode.mode {
            AddressingMode::Immediate => {
                let carry = self.register_a & 1 != 0;
                let value = self.register_a >> 1;
                self.set_register_a(value);
                self.status.set(Flags::CARRY, carry);
            }
            _ => {
                let (address, _) = self.get_parameter_address(&opcode.mode);
                let mut value = self.fetch_byte(address);
                let carry = value & 1 != 0;
                value >>= 1;
                self.store_byte(address, value);
                self.update_zero_and_negative_flags(value);
                self.status.set(Flags::CARRY, carry);
            }
        }
    }

    fn rol(&mut self, opcode: &opcodes::Opcode) {
        // Rotate Left through carry flag
        let curr_carry = self.status.contains(Flags::CARRY);
        match opcode.mode {
            AddressingMode::Immediate => {
                let (value, new_carry) = rotate_value_left(self.register_a, curr_carry);
                self.set_register_a(value);
                self.status.set(Flags::CARRY, new_carry);
            }
            _ => {
                let (address, _) = self.get_parameter_address(&opcode.mode);
                let value = self.fetch_byte(address);
                let (result, new_carry) = rotate_value_left(value, curr_carry);
                self.store_byte(address, result);
                self.update_zero_and_negative_flags(result);
                self.status.set(Flags::CARRY, new_carry);
            }
        }
    }

    fn ror(&mut self, opcode: &opcodes::Opcode) {
        // Rotate Right through carry flag
        let curr_carry = self.status.contains(Flags::CARRY);
        match opcode.mode {
            AddressingMode::Immediate => {
                let (value, new_carry) = rotate_value_right(self.register_a, curr_carry);
                self.set_register_a(value);
                self.status.set(Flags::CARRY, new_carry);
            }
            _ => {
                let (address, _) = self.get_parameter_address(&opcode.mode);
                let value = self.fetch_byte(address);
                let (result, new_carry) = rotate_value_right(value, curr_carry);
                self.store_byte(address, result);
                self.update_zero_and_negative_flags(result);
                self.status.set(Flags::CARRY, new_carry);
            }
        }
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

    fn jmp(&mut self, opcode: &opcodes::Opcode) {
        let (address, _) = self.get_parameter_address(&opcode.mode);
        self.set_program_counter(address);
    }

    fn jsr(&mut self, opcode: &opcodes::Opcode) {
        // Jump to Subroutine
        let (jump_address, _) = self.get_parameter_address(&opcode.mode);
        let return_address = self.program_counter.wrapping_add(1);
        self.stack_push_u16(return_address);
        self.set_program_counter(jump_address);
    }

    fn rts(&mut self) {
        // Return from Subroutine
        let return_address_minus_one = self.stack_pop_u16();
        let address = return_address_minus_one.wrapping_add(1);
        self.set_program_counter(address);
    }

    fn rti(&mut self) {
        // Return from Interrupt
        // NOTE: Note that unlike RTS, the return address on the stack is the actual address rather than the address-1
        self.plp(); // pop stack into status flags
        let return_address = self.stack_pop_u16();
        self.set_program_counter(return_address);
        self.status.set(Flags::BREAK, false);
        self.status.set(Flags::BREAK2, true);
    }

    fn bne(&mut self, opcode: &opcodes::Opcode) {
        // Branch if ZERO is clear
        self.branch(opcode, self.status.contains(Flags::ZERO) == false)
    }

    fn bvs(&mut self, opcode: &opcodes::Opcode) {
        // Branch if OVERFLOW is set
        self.branch(opcode, self.status.contains(Flags::OVERFLOW))
    }
    fn bvc(&mut self, opcode: &opcodes::Opcode) {
        // Branch if OVERFLOW is clear
        self.branch(opcode, self.status.contains(Flags::OVERFLOW) == false)
    }

    fn bmi(&mut self, opcode: &opcodes::Opcode) {
        // Branch if NEGATIVE is set
        self.branch(opcode, self.status.contains(Flags::NEGATIVE))
    }

    fn beq(&mut self, opcode: &opcodes::Opcode) {
        // Branch if ZERO is set
        self.branch(opcode, self.status.contains(Flags::ZERO))
    }

    fn bcs(&mut self, opcode: &opcodes::Opcode) {
        // Branch if CARRY is set
        self.branch(opcode, self.status.contains(Flags::CARRY))
    }

    fn bcc(&mut self, opcode: &opcodes::Opcode) {
        // Branch if CARRY is clear
        self.branch(opcode, self.status.contains(Flags::CARRY) == false)
    }

    fn bpl(&mut self, opcode: &opcodes::Opcode) {
        // Branch if NEGATIVE is clear
        self.branch(opcode, self.status.contains(Flags::NEGATIVE) == false)
    }

    fn bit(&mut self, opcode: &opcodes::Opcode) {
        // Bit Test
        let (address, _) = self.get_parameter_address(&opcode.mode);
        let mut value = self.bus.fetch_byte(address);
        value &= self.register_a;
        self.status.set(Flags::ZERO, value == 0);
        self.status.set(Flags::NEGATIVE, value & 1 << 7 != 0);
        self.status.set(Flags::OVERFLOW, value & 1 << 6 != 0);
    }

    /////////////////////////
    /// Unofficial Opcodes
    /////////////////////////

    fn sax(&mut self, opcode: &opcodes::Opcode) {
        // SAX => A AND X -> M
        /* A and X are put on the bus at the same time (resulting effectively
          in an AND operation) and stored in M
        */
        let (address, _) = self.get_parameter_address(&opcode.mode);
        let result = self.register_a & self.register_x;
        self.store_byte(address, result);
    }

    fn sbx(&mut self, opcode: &opcodes::Opcode) {
        // TODO: test this...
        // SBX (AXS, SAX) => CMP and DEX at once, sets flags like CMP
        let (address, _) = self.get_parameter_address(&opcode.mode);
        let value = self.fetch_byte(address);
        let and_result = self.register_a & self.register_x;
        let result = and_result.wrapping_sub(value);
        self.status.set(Flags::CARRY, result >= value);
        self.update_zero_and_negative_flags(and_result);
    }

    fn anc(&mut self, opcode: &opcodes::Opcode) {
        let (address, _) = self.get_parameter_address(&opcode.mode);
        let value = self.fetch_byte(address);
        self.set_register_a(self.register_a & value);
        self.status
            .set(Flags::CARRY, self.register_a & 0b0100_0000 != 0);
    }
}

fn is_boundary_crossed(addr1: u16, addr2: u16) -> bool {
    addr1 & 0xFF00 != addr2 & 0xFF00
}

fn rotate_value_left(value: u8, current_carry: bool) -> (u8, bool) {
    let new_carry = value & 0b1000_0000 != 0;
    let mut shifted = value << 1;
    shifted |= current_carry as u8;
    (shifted, new_carry)
}

fn rotate_value_right(value: u8, current_carry: bool) -> (u8, bool) {
    let new_carry = value & 0b0000_0001 != 0;
    let mut shifted = value >> 1;
    shifted |= (current_carry as u8) << 7;
    (shifted, new_carry)
}

#[cfg(test)]
mod test {
    use super::*;

    fn init_cpu() -> CPU {
        let bus = Bus::new();
        CPU::new(bus)
    }

    #[async_std::test]
    async fn test_0xaa_tax_0xa8_tay() {
        let program = &[
            0xa9, // LDA immediate
            0x42, //    with $0F
            0xAA, // TAX
            0xA8, // TAY
            0x00, // BRK
        ];
        let mut cpu = init_cpu();
        cpu.load(program);
        cpu.run().await;
        assert_eq!(cpu.register_x, 0x42);
        assert_eq!(cpu.register_y, 0x42);
        assert_eq!(cpu.status.contains(Flags::ZERO), false);
        assert_eq!(cpu.status.contains(Flags::NEGATIVE), false);
    }

    #[async_std::test]
    async fn test_0xa9_lda_immediate_load_data() {
        let program = &[
            0xa9, // LDA immediate
            0x05, //    with $05
            0x00, // BRK
        ];
        let mut cpu = init_cpu();
        cpu.load(program);
        cpu.run().await;
        assert_eq!(cpu.register_a, 0x05);
        assert_eq!(cpu.status.contains(Flags::ZERO), false);
        assert_eq!(cpu.status.contains(Flags::NEGATIVE), false);
    }

    #[async_std::test]
    async fn test_0xa9_lda_zero_flag() {
        let program = &[
            0xa9, // LDA immediate
            0x00, //    with $0
            0x00, // BRK
        ];
        let mut cpu = init_cpu();
        cpu.load(program);
        cpu.run().await;
        assert_eq!(cpu.status.contains(Flags::ZERO), true);
    }

    #[async_std::test]
    async fn test_0xa5_lda_zero_page_load_data() {
        let program = &[
            0xa5, // LDA ZeroPage
            0x05, //    with $05
            0x00, // BRK
        ];
        let mut cpu = init_cpu();
        cpu.load(program);
        cpu.bus.store_byte(0x05, 0x42);
        cpu.run().await;
        assert_eq!(cpu.register_a, 0x42);
        assert_eq!(cpu.status.contains(Flags::ZERO), false);
        assert_eq!(cpu.status.contains(Flags::NEGATIVE), false);
    }

    #[async_std::test]
    async fn test_0xa5_lda_zero_page_x_load_data() {
        let program = &[
            0xa9, // LDA immediate
            0x0F, //    with $0F
            0xAA, // TAX
            0xB5, // LDA ZeroPageX
            0x80, //    with $80        - X = $0F, loading A with data from $8F = 0x42
            0x00, // BRK
        ];
        let mut cpu = init_cpu();
        cpu.load(program);
        cpu.bus.store_byte(0x8F, 0x42);
        cpu.run().await;
        assert_eq!(cpu.register_a, 0x42);
        assert_eq!(cpu.register_x, 0x0F);
        assert_eq!(cpu.status.contains(Flags::ZERO), false);
        assert_eq!(cpu.status.contains(Flags::NEGATIVE), false);
    }

    #[async_std::test]
    async fn test_0xb5_lda_absolute_load_data() {
        let program = &[
            0xAD, // LDA absolute (5 cycles)
            0xEF, //
            0xBE, // Loading from little endian $EFBE which will actually be $BEEF
            0xAA, // TAX (1 cycle)
            0x00, // BRK
        ];
        let mut cpu = init_cpu();
        cpu.load(program);
        cpu.bus.store_byte(0xBEEF, 0x42);
        cpu.run().await;
        assert_eq!(cpu.register_a, 0x42);
        assert_eq!(cpu.register_x, 0x42);
        assert_eq!(cpu.bus.cycles, 5 + 1);
        assert_eq!(cpu.status.contains(Flags::ZERO), false);
        assert_eq!(cpu.status.contains(Flags::NEGATIVE), false);
    }

    #[async_std::test]
    async fn test_set_flags() {
        let program = &[
            0x38, // SEC
            0x78, // SEI
            0xF8, // SED
            0x00, // BRK
        ];
        let mut cpu = init_cpu();
        cpu.load(program);
        cpu.run().await;
        assert_eq!(cpu.status.contains(Flags::CARRY), true);
        assert_eq!(cpu.status.contains(Flags::INTERRUPT_DISABLE), true);
        assert_eq!(cpu.status.contains(Flags::DECIMAL_MODE), true);
    }

    #[async_std::test]
    async fn test_set_and_clear_flags() {
        let program = &[
            0x38, // SEC
            0x78, // SEI
            0xF8, // SED
            0x18, // CLC
            0x58, // CLI
            0xD8, // CLD
            0x00, // BRK
        ];
        let mut cpu = init_cpu();
        cpu.load(program);
        cpu.run().await;
        assert_eq!(cpu.status.contains(Flags::CARRY), false);
        assert_eq!(cpu.status.contains(Flags::INTERRUPT_DISABLE), false);
        assert_eq!(cpu.status.contains(Flags::DECIMAL_MODE), false);
    }

    #[async_std::test]
    async fn test_adc_without_carry() {
        let program = &[
            0xA9, // LDA
            0x10, //   with 0x10
            0x69, // ADC
            0x07, //   with 0x07
            0x00, // BRK
        ];
        let mut cpu = init_cpu();
        cpu.load(program);
        cpu.run().await;
        assert_eq!(cpu.register_a, 0x17);
        assert_eq!(cpu.status.contains(Flags::CARRY), false);
        assert_eq!(cpu.status.contains(Flags::OVERFLOW), false);
    }

    #[async_std::test]
    async fn test_adc_with_overflow() {
        let program = &[
            0xA9, // LDA
            0x7F, //   with 0x7F
            0x69, // ADC
            0x0F, //   with 0x0F
            0x00, // BRK
        ];
        let mut cpu = init_cpu();
        cpu.load(program);
        cpu.run().await;
        assert_eq!(cpu.status.contains(Flags::CARRY), false);
        assert_eq!(cpu.status.contains(Flags::OVERFLOW), true);
        assert_eq!(cpu.register_a, 0x8E);
    }

    #[async_std::test]
    async fn test_adc_with_carry() {
        let program = &[
            0xA9, // LDA
            0xFF, //   with 0xFF
            0x69, // ADC
            0x0F, //   with 0x01
            0x00, // BRK
        ];
        let mut cpu = init_cpu();
        cpu.load(program);
        cpu.run().await;
        assert_eq!(cpu.status.contains(Flags::CARRY), true);
        assert_eq!(cpu.status.contains(Flags::OVERFLOW), false);
        assert_eq!(cpu.register_a, 0x0E);
    }

    #[async_std::test]
    async fn test_sbc_without_borrow() {
        let program = &[
            0xA9, // LDA
            0xFF, //   with 0xFF
            0x38, // SEC
            0xE9, // SBC
            0x0F, //   with 0x0F
            0x00, // BRK
        ];
        let mut cpu = init_cpu();
        cpu.load(program);
        cpu.run().await;
        // Note: In SBC, the "CARRY" flag becomes a "BORROW" flag which is complement
        assert_eq!(cpu.status.contains(Flags::CARRY), true);
        assert_eq!(cpu.status.contains(Flags::OVERFLOW), false);
        assert_eq!(cpu.register_a, 0xF0);
    }

    #[async_std::test]
    async fn test_sbc_with_borrow() {
        let program = &[
            0xA9, // LDA
            0x00, //   with 0x00
            0x38, // SEC -- Note: it's standard to SEC before any SBC (complement of carry acts as borrow flag)
            0xE9, // SBC
            0x01, //   with 0x01
            0x00, // BRK
        ];
        let mut cpu = init_cpu();
        cpu.load(program);
        cpu.run().await;
        assert_eq!(cpu.status.contains(Flags::CARRY), false);
        assert_eq!(cpu.status.contains(Flags::OVERFLOW), false);
        assert_eq!(cpu.register_a, 0xFF);
    }

    #[test]
    fn test_rotate_value_right() {
        let carry = true;
        let value = 0xE0;
        let (result, new_carry) = rotate_value_right(value, carry);
        assert_eq!(result, 240);
        assert_eq!(new_carry, false);
    }

    #[test]
    fn test_rotate_value_left() {
        let carry = true;
        let value = 0xE0;
        let (result, new_carry) = rotate_value_left(value, carry);
        assert_eq!(result, 193);
        assert_eq!(new_carry, true);
    }

    #[async_std::test]
    async fn test_0x8a_txa() {
        let program = &[
            0x8A, // TXA
            0x00, // BRK
        ];
        let mut cpu = init_cpu();
        cpu.load(program);
        cpu.set_register_x(0x42);
        cpu.run().await;
        assert_eq!(cpu.register_a, 0x42);
    }

    #[async_std::test]
    async fn test_0x98_tya() {
        let program = &[
            0x98, // TYA
            0x00, // BRK
        ];
        let mut cpu = init_cpu();
        cpu.load(program);
        cpu.set_register_y(0x88);
        cpu.run().await;
        assert_eq!(cpu.register_a, 0x88);
    }

    #[async_std::test]
    async fn test_0xba_tsx() {
        let program = &[
            0xBA, // TSX
            0x00, // BRK
        ];
        let mut cpu = init_cpu();
        cpu.load(program);
        cpu.stack_pointer = 0x37;
        cpu.run().await;
        assert_eq!(cpu.register_x, 0x37);
    }

    #[async_std::test]
    async fn test_0x9a_txs() {
        let program = &[
            0x9A, // TXS
            0x00, // BRK
        ];
        let mut cpu = init_cpu();
        cpu.load(program);
        cpu.register_x = 0x33;
        cpu.run().await;
        assert_eq!(cpu.stack_pointer, 0x33);
    }

    #[async_std::test]
    async fn test_0xd0_bne_success() {
        let program = &[
            0xD0, // BNE
            0x0F, //   to 0x0F
            0x00, // BRK
        ];
        let mut cpu = init_cpu();
        cpu.load(program);
        cpu.status.set(Flags::ZERO, false);
        cpu.run().await;
        let want = 0x8012;
        assert_eq!(
            cpu.program_counter,
            want,
            "{}",
            format!(
                "program_counter mismatch - Got: ${:02X} Want: ${:02X}",
                cpu.program_counter, want
            )
        );
    }

    #[async_std::test]
    async fn test_0xd0_bne_failed() {
        let program = &[
            0xD0, // BNE
            0x0F, //   to 0x0F
            0x00, // BRK
        ];
        let mut cpu = init_cpu();
        cpu.load(program);
        cpu.status.set(Flags::ZERO, true);
        cpu.run().await;
        let want = 0x8003;
        assert_eq!(
            cpu.program_counter,
            want,
            "{}",
            format!(
                "program_counter mismatch - Got: ${:02X} Want: ${:02X}",
                cpu.program_counter, want
            )
        );
    }
}
