use super::interrupts::{Interrupt, InterruptType};
use super::{interrupts, opcodes};
use crate::trace;
use bitflags::bitflags;
use std::collections::HashMap;

// const DEBUG: bool = true;
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
    Indirect, // Exclusive to JMP opcodes
    Relative, // Exclusive to Branch opcodes
    None,
}

#[derive(Debug)]
pub enum CpuMode {
    Read,
    Write,
}

pub trait CpuBusInterface {
    fn cpu_bus_read(&mut self, addr: u16) -> u8;
    fn cpu_bus_write(&mut self, addr: u16, value: u8);
}

pub struct CPU {
    pub bus: Option<*mut dyn CpuBusInterface>,
    pub cycles: usize,
    pub cpu_mode: CpuMode,
    pub rdy: bool,
    pub halt_scheduled: bool,

    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub stack_pointer: u8,
    pub status: Flags,
    pub program_counter: u16,

    pub skip_cycles: u8,
    extra_cycles: u8,
    skip_pc_advance: bool,

    nmi_pending: bool,
    interrupt_stack: Vec<InterruptType>,

    pub last_opcode_desc: String,
    // pub tracer: Tracer,
}

impl CPU {
    pub fn new() -> CPU {
        let cpu = CPU {
            bus: None,
            cycles: 0,
            cpu_mode: CpuMode::Read,
            halt_scheduled: false,
            rdy: true,
            register_a: 0,
            register_x: 0,
            register_y: 0,
            stack_pointer: CPU_STACK_RESET,
            status: Flags::from_bits_truncate(0b0010_0010),
            program_counter: CPU_PC_RESET,
            skip_cycles: 0,
            extra_cycles: 0,
            skip_pc_advance: false,
            // tracer: Tracer::new(128),
            nmi_pending: false,
            interrupt_stack: vec![],
            last_opcode_desc: "".to_string(),
        };
        cpu
    }

    /// `connect_bus` MUST be called after constructing CPU
    pub fn connect_bus(&mut self, bus: *mut dyn CpuBusInterface) {
        self.bus = Some(bus);
        self.program_counter = self.bus_read_u16(0xFFFC);
    }

    /// `bus_read` is safe because Bus owns CPU
    pub fn bus_read(&self, addr: u16) -> u8 {
        unsafe { (*self.bus.unwrap()).cpu_bus_read(addr) }
    }

    pub fn bus_read_u16(&self, addr: u16) -> u16 {
        let lo = self.bus_read(addr) as u16;
        let hi = self.bus_read(addr.wrapping_add(1)) as u16;
        (hi << 8) | lo
    }

    /// `bus_write` is safe because Bus owns CPU
    pub fn bus_write(&self, addr: u16, data: u8) {
        unsafe {
            (*self.bus.unwrap()).cpu_bus_write(addr, data);
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
        self.skip_cycles = 0;
        self.skip_pc_advance = false;

        self.nmi_pending = false; // PPU will notify CPU when NMI needs handling
        self.interrupt_stack = vec![]; // This prevents nested NMI (while allowing nested BRKs)
    }

    #[allow(dead_code)]
    pub fn run(&mut self) {
        loop {
            let (_, _, should_break) = self.tick();
            if should_break {
                break;
            }
        }
    }

    pub fn trigger_nmi(&mut self) {
        self.nmi_pending = true;
    }

    fn toggle_mode(&mut self) {
        self.cpu_mode = match &self.cpu_mode {
            CpuMode::Read => CpuMode::Write,
            CpuMode::Write => CpuMode::Read,
        };
    }

    // `tick` returns (num_cycles, bytes_consumed, is_breaking)
    pub fn tick(&mut self) -> (u8, u8, bool) {
        // Stall for previous cycles from last instruction
        if self.skip_cycles > 0 {
            self.skip_cycles -= 1;
            self.toggle_mode();
            self.cycles += 1;
            return (0, 0, false);
        }

        // DMAs schedule halts, which triggers a set of events:
        // - CPU finishes current instruction (above)
        // - CPU waits for "Read" state
        // - CPU halts for 1 cycle to enter DMA mode
        if self.halt_scheduled {
            match self.cpu_mode {
                CpuMode::Read => {
                    self.rdy = false;
                    self.halt_scheduled = false;
                }
                CpuMode::Write => {
                    trace!("OAM DMA DUMMY READ");
                    self.toggle_mode();
                }
            }
            self.cycles += 1;
            return (0, 0, false);
        }

        self.toggle_mode();

        // If we're not already handling NMI, immediately handle it
        if !self.interrupt_stack.contains(&InterruptType::NMI) && self.nmi_pending {
            self.nmi_pending = false;
            self.handle_interrupt(interrupts::NMI);
        }

        let ref opcodes: HashMap<u8, &'static opcodes::Opcode> = *opcodes::OPCODES_MAP;

        self.extra_cycles = 0;
        self.skip_pc_advance = false;
        let code = self.bus_read(self.program_counter);
        let opcode_lookup = opcodes.get(&code);
        let opcode = match opcode_lookup {
            Some(opcode) => *opcode,
            None => {
                // self.tracer.print_trace();
                panic!("Unknown opcode: {:02X}", &code);
            }
        };

        {
            // Build debug trace
            let mut operand_bytes: Vec<u8> = vec![];
            for i in 1..opcode.size {
                let address = self.program_counter.wrapping_add(i as u16);
                operand_bytes.push(self.bus_read(address));
            }
            let trace = format!(
                "({}) PC:${:04X} SP:${:02X} A:${:02X} X:${:02X} Y:${:02X} status: 0b{:08b} \tOpcode: (${:02X}) {} {:02X?}",
                self.program_counter,
                self.program_counter,
                self.stack_pointer,
                self.register_a,
                self.register_x,
                self.register_y,
                self.status.bits(),
                self.bus_read(self.program_counter),
                opcode.name,
                operand_bytes
            );
            self.last_opcode_desc = format!("Opcode: {} {:02x?}", opcode.name, operand_bytes);
            if DEBUG {
                println!("{trace}");
            }
            // trace!("{}", format!("CPU: {}", trace));
            // self.tracer.write(trace);
        }

        self.program_counter = self.program_counter.wrapping_add(1);

        match code {
            0x00 => self.brk(), // BRK
            0xEA => {}          // NOP

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

            0x24 | 0x2C => self.bit(opcode), // BIT

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
            // Illegal Opcodes
            /////////////////////////
            0xC7 | 0xD7 | 0xCF | 0xDF | 0xDB | 0xD3 | 0xC3 => {
                // DCP => DEC oper + CMP oper
                let dec_value = self.dec(opcode);

                // Compare register_a with decremented value
                let result = self.register_a.wrapping_sub(dec_value);
                self.status.set(Flags::CARRY, self.register_a >= dec_value);
                self.update_zero_and_negative_flags(result);
                self.extra_cycles = 0;
            }
            0x27 | 0x37 | 0x2F | 0x3F | 0x3B | 0x33 | 0x23 => {
                // RLA => ROL oper + AND oper
                self.rla(opcode);
            }
            0x07 | 0x17 | 0x0F | 0x1F | 0x1B | 0x03 | 0x13 => {
                // SLO => ASL oper + ORA oper
                self.slo(opcode);
            }
            0x47 | 0x57 | 0x4F | 0x5F | 0x5B | 0x43 | 0x53 => {
                // SRE => LSR oper + EOR oper
                self.sre(opcode);
            }
            0x67 | 0x77 | 0x6F | 0x7F | 0x7B | 0x63 | 0x73 => {
                // RRA => ROR oper + ADC oper
                self.ror(opcode);
                self.adc(opcode);
                self.extra_cycles = 0;
            }
            0xE7 | 0xF7 | 0xEF | 0xFF | 0xFB | 0xE3 | 0xF3 => {
                // ISC (ISB / INS) => INC oper + SBC oper
                let inc_result = self.inc(opcode);
                self.sub_from_register_a(inc_result);
            }
            0xA7 | 0xB7 | 0xAF | 0xBF | 0xA3 | 0xB3 => {
                // LAX => LDA oper + LDX oper
                self.lda(opcode);
                self.ldx(opcode);
                if self.extra_cycles == 2 {
                    self.extra_cycles = 1;
                }
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
                // ARR => AND oper + ROR (Plus some wonky flag manipulation)
                self.arr(opcode);
            }
            0xEB => {
                // USBC (SBC) => SBC oper + NOP
                self.sbc(opcode);
            }
            0x0B | 0x2B => {
                // ANC => A AND oper, bit(7) -> C
                self.anc(opcode);
            }
            0x4B => {
                // ALR => AND oper + LSR
                self.and(opcode);
                self.lsr(opcode);
            }
            0xBB => {
                // LAS (LAR) => LDA + AND with SP, store in A, X, SP
                self.las(opcode);
            }
            0x02 => {
                // JAM - This freezes the CPU
                // NOTE: I'm hijacking this opcode for use in processor_tests
                //       0x02 now breaks the normal run() loop{}
                self.cycles += 1;
                return (11, 1, true);
            }
            0x12 | 0x22 | 0x32 | 0x42 | 0x52 | 0x62 | 0x72 | 0x92 | 0xB2 | 0xD2 | 0xF2 => {
                // JAM - These instructions freeze the CPU
                panic!(
                    "{}",
                    &format!("JAM instruction 0x{:02X} freezes CPU!", opcode.value)
                )
            }

            0x8B | 0xAB | 0x9F | 0x93 | 0x9E | 0x9C | 0x9B => {
                // Unstable and highly-unstable opcodes (Purposely unimplemented)
                panic!(
                    "{}",
                    &format!(
                        "Instruction 0x{:02X} unimplemented. It's too unstable!",
                        opcode.value
                    )
                )
            }

            0x1C | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC => {
                // page-crossing NOPs
                self.nop_page_cross(opcode);
            }

            0x80 | 0x82 | 0x89 | 0xC2 | 0xE2 | 0x04 | 0x44 | 0x64 | 0x14 | 0x34 | 0x54 | 0x74
            | 0xD4 | 0xF4 | 0x0C | 0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xFA => {
                // Various single and multiple-byte NOPs
            } // _ => unreachable!()
        }

        // Tick the bus for opcode cycles. Add any extra cycles from boundary_crosses and other special cases
        let cycle_count = opcode.cycles + self.extra_cycles;

        // Subtract one to account for execution of the current cycle
        self.skip_cycles = cycle_count - 1;

        // Advance PC normally if an opcode didn't modify it
        if !self.skip_pc_advance {
            self.program_counter = self.program_counter.wrapping_add((opcode.size - 1) as u16);
        }
        self.cycles += 1;
        (cycle_count, opcode.size, false)
    }

    // Utility functions
    /////////////////////
    fn get_parameter_address(&mut self, mode: &AddressingMode) -> (u16, bool) {
        match mode {
            AddressingMode::Absolute => (self.bus_read_u16(self.program_counter), false),
            AddressingMode::Immediate => (self.program_counter, false),
            AddressingMode::ZeroPage => (self.bus_read(self.program_counter) as u16, false),
            AddressingMode::ZeroPageX => {
                let base = self.bus_read(self.program_counter);
                let addr = base.wrapping_add(self.register_x) as u16;
                (addr, false)
            }
            AddressingMode::ZeroPageY => {
                let base = self.bus_read(self.program_counter);
                let addr = base.wrapping_add(self.register_y) as u16;
                (addr, false)
            }
            AddressingMode::AbsoluteX => {
                let base = self.bus_read_u16(self.program_counter);
                let addr = base.wrapping_add(self.register_x as u16);

                // Only read from base page (not the final address)
                let dummy_addr =
                    (base & 0xFF00) | ((base.wrapping_add(self.register_y as u16)) & 0x00FF);
                let _ = self.bus_read(dummy_addr);

                (addr, is_boundary_crossed(base, addr))
            }
            AddressingMode::AbsoluteY => {
                let base = self.bus_read_u16(self.program_counter);
                let addr = base.wrapping_add(self.register_y as u16);

                // Only read from base page (not the final address)
                let dummy_addr =
                    (base & 0xFF00) | ((base.wrapping_add(self.register_y as u16)) & 0x00FF);
                let _ = self.bus_read(dummy_addr);

                (addr, is_boundary_crossed(base, addr))
            }
            AddressingMode::IndirectX => {
                let base = self.bus_read(self.program_counter);
                let addr = base.wrapping_add(self.register_x); // Zero-page wrapping
                let lo = self.bus_read(addr as u16) as u16;
                let hi = self.bus_read(addr.wrapping_add(1) as u16) as u16; // Zero-page wrap +1 as well
                (hi << 8 | lo, false)
            }
            AddressingMode::IndirectY => {
                let base = self.bus_read(self.program_counter) as u16;
                let lo = self.bus_read(base) as u16;
                let hi = self.bus_read((base as u8).wrapping_add(1) as u16) as u16;
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
                let indirect_vec = self.bus_read_u16(self.program_counter);
                let address = if indirect_vec & 0x00FF == 0x00FF {
                    let lo = self.bus_read(indirect_vec) as u16;
                    let hi = self.bus_read(indirect_vec & 0xFF00) as u16;
                    (hi << 8) | lo
                } else {
                    self.bus_read_u16(indirect_vec)
                };
                (address, false)
            }
            AddressingMode::Relative => {
                // Note: Branch opcodes exclusively use this address mode
                let offset = self.bus_read(self.program_counter) as i8; // sign-extend u8 to i8
                let base_pc = self.program_counter.wrapping_add(1); // the relative address is based on a PC /after/ the current opcode
                let target_address = base_pc.wrapping_add_signed(offset as i16);
                let boundary_crossed = is_boundary_crossed(base_pc, target_address);
                (target_address, boundary_crossed)
            }
            _ => unimplemented!(),
        }
    }

    fn set_register_a(&mut self, value: u8) {
        self.register_a = value;
        self.update_zero_and_negative_flags(value);
    }

    pub(crate) fn set_register_x(&mut self, value: u8) {
        self.register_x = value;
        self.update_zero_and_negative_flags(value);
    }

    pub(crate) fn set_register_y(&mut self, value: u8) {
        self.register_y = value;
        self.update_zero_and_negative_flags(value);
    }

    fn set_program_counter(&mut self, address: u16) {
        self.program_counter = address;
        self.skip_pc_advance = true;
    }

    fn stack_push(&mut self, value: u8) {
        let address = CPU_STACK_BASE.wrapping_add(self.stack_pointer as u16);
        self.bus_write(address, value);
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
        self.bus_read(CPU_STACK_BASE.wrapping_add(self.stack_pointer as u16))
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
        let value = self.bus_read(address);
        self.status.set(Flags::CARRY, compare_value >= value);
        self.update_zero_and_negative_flags(compare_value.wrapping_sub(value));
        self.extra_cycles += boundary_crossed as u8;
    }

    fn branch(&mut self, opcode: &opcodes::Opcode, condition: bool) {
        let (address, boundary_crossed) = self.get_parameter_address(&opcode.mode);
        let cycles = boundary_crossed as u8;
        if condition {
            self.set_program_counter(address);
            self.extra_cycles = self.extra_cycles + cycles + 1;
        }
    }

    fn handle_interrupt(&mut self, interrupt: Interrupt) {
        // TODO: remove this sanity check
        if interrupt.interrupt_type == InterruptType::NMI
            && self.interrupt_stack.contains(&InterruptType::NMI)
        {
            panic!("Error: Nested NMI detected! This should be impossible");
        }

        self.interrupt_stack.push(interrupt.interrupt_type);

        self.stack_push_u16(self.program_counter);

        let mut status_flags = Flags::from_bits_truncate(self.status.bits());
        status_flags.set(Flags::BREAK, interrupt.b_flag_mask & 0b0001_0000 != 0);
        status_flags.set(Flags::BREAK2, interrupt.b_flag_mask & 0b0010_0000 != 0);
        self.stack_push(status_flags.bits());

        // TODO: What does this affect?
        self.status.set(Flags::INTERRUPT_DISABLE, true); // Disable interrupts while handling one

        self.extra_cycles += interrupt.cpu_cycles;
        let jmp_address = self.bus_read_u16(interrupt.vector_addr);
        self.set_program_counter(jmp_address);
    }

    // Opcodes
    /////////////
    fn lda(&mut self, opcode: &opcodes::Opcode) {
        let (address, boundary_cross) = self.get_parameter_address(&opcode.mode);
        self.extra_cycles += boundary_cross as u8; // boundary_cross adds 1 extra cycle

        let param = self.bus_read(address);
        self.set_register_a(param);
    }

    fn ldx(&mut self, opcode: &opcodes::Opcode) {
        let (address, boundary_cross) = self.get_parameter_address(&opcode.mode);
        self.extra_cycles += boundary_cross as u8; // boundary_cross adds 1 extra cycle

        let param = self.bus_read(address);
        self.set_register_x(param);
    }

    fn ldy(&mut self, opcode: &opcodes::Opcode) {
        let (address, boundary_cross) = self.get_parameter_address(&opcode.mode);
        self.extra_cycles += boundary_cross as u8; // boundary_cross adds 1 extra cycle

        let param = self.bus_read(address);
        self.set_register_y(param);
    }

    fn sta(&mut self, opcode: &opcodes::Opcode) {
        let (address, _) = self.get_parameter_address(&opcode.mode);
        // self.bus_read(address);
        self.bus_write(address, self.register_a);
    }

    fn stx(&mut self, opcode: &opcodes::Opcode) {
        let (address, _) = self.get_parameter_address(&opcode.mode);
        self.bus_write(address, self.register_x);
    }

    fn sty(&mut self, opcode: &opcodes::Opcode) {
        let (address, _) = self.get_parameter_address(&opcode.mode);
        self.bus_write(address, self.register_y);
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
        self.status.remove(Flags::BREAK); // This flag is disabled when fetching
        self.status.insert(Flags::BREAK2); // This flag is supposed to always be 1 on CPU
    }

    fn asl(&mut self, opcode: &opcodes::Opcode) -> u8 {
        // Arithmetic Shift Left into carry
        match opcode.mode {
            AddressingMode::Immediate => {
                let carry = self.register_a & 0x80 != 0;
                let value = self.register_a << 1;
                self.set_register_a(value);
                self.status.set(Flags::CARRY, carry);
                value
            }
            _ => {
                let (address, _) = self.get_parameter_address(&opcode.mode);
                let mut value = self.bus_read(address);
                let carry = value & 0x80 != 0;
                value <<= 1;
                self.bus_write(address, value);
                self.update_zero_and_negative_flags(value);
                self.status.set(Flags::CARRY, carry);
                value
            }
        }
    }

    fn lsr(&mut self, opcode: &opcodes::Opcode) -> u8 {
        // Logical Shift Right into carry
        match opcode.mode {
            AddressingMode::Immediate => {
                let carry = self.register_a & 1 != 0;
                let value = self.register_a >> 1;
                self.set_register_a(value);
                self.status.set(Flags::CARRY, carry);
                value
            }
            _ => {
                let (address, _) = self.get_parameter_address(&opcode.mode);
                let mut value = self.bus_read(address);
                let carry = value & 1 != 0;
                value >>= 1;
                self.bus_write(address, value);
                self.update_zero_and_negative_flags(value);
                self.status.set(Flags::CARRY, carry);
                value
            }
        }
    }

    fn rol(&mut self, opcode: &opcodes::Opcode) -> u8 {
        // Rotate Left through carry flag
        let curr_carry = self.status.contains(Flags::CARRY);
        match opcode.mode {
            AddressingMode::Immediate => {
                let (value, new_carry) = rotate_value_left(self.register_a, curr_carry);
                self.set_register_a(value);
                self.status.set(Flags::CARRY, new_carry);
                value
            }
            _ => {
                let (address, _) = self.get_parameter_address(&opcode.mode);
                let value = self.bus_read(address);
                let (result, new_carry) = rotate_value_left(value, curr_carry);
                self.bus_write(address, result);
                self.update_zero_and_negative_flags(result);
                self.status.set(Flags::CARRY, new_carry);
                result
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
                let value = self.bus_read(address);
                let (result, new_carry) = rotate_value_right(value, curr_carry);
                self.bus_write(address, result);
                self.update_zero_and_negative_flags(result);
                self.status.set(Flags::CARRY, new_carry);
            }
        }
    }

    fn inc(&mut self, opcode: &opcodes::Opcode) -> u8 {
        // Increment value at Memory
        let (address, _) = self.get_parameter_address(&opcode.mode);
        let mut value = self.bus_read(address);
        value = value.wrapping_add(1);
        self.bus_write(address, value);
        self.update_zero_and_negative_flags(value);
        value
    }

    fn dec(&mut self, opcode: &opcodes::Opcode) -> u8 {
        // Decrement value at Memory
        let (address, _) = self.get_parameter_address(&opcode.mode);
        let mut value = self.bus_read(address);
        value = value.wrapping_sub(1);
        self.bus_write(address, value);
        self.update_zero_and_negative_flags(value);
        value
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
        let value = self.bus_read(address);
        self.add_to_register_a(value);
        self.extra_cycles += boundary_crossed as u8;
    }

    fn sbc(&mut self, opcode: &opcodes::Opcode) {
        // Subtract with Carry
        let (address, boundary_crossed) = self.get_parameter_address(&opcode.mode);
        let value = self.bus_read(address);
        self.sub_from_register_a(value);
        self.extra_cycles += boundary_crossed as u8;
    }

    fn and(&mut self, opcode: &opcodes::Opcode) {
        // Logical AND on accumulator
        let (address, boundary_crossed) = self.get_parameter_address(&opcode.mode);
        let value = self.bus_read(address);
        self.set_register_a(self.register_a & value);
        self.extra_cycles += boundary_crossed as u8;
    }

    fn eor(&mut self, opcode: &opcodes::Opcode) {
        // Logical Exclusive OR on accumulator
        let (address, boundary_crossed) = self.get_parameter_address(&opcode.mode);
        let value = self.bus_read(address);
        self.set_register_a(self.register_a ^ value);
        self.extra_cycles += boundary_crossed as u8;
    }

    fn ora(&mut self, opcode: &opcodes::Opcode) {
        // Logical OR on accumulator
        let (address, boundary_crossed) = self.get_parameter_address(&opcode.mode);
        let value = self.bus_read(address);
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

        let _ = self.bus_read(self.program_counter); // dummy read
        self.set_program_counter(address);
    }

    fn rti(&mut self) {
        // Return from Interrupt
        // NOTE: Note that unlike RTS, the return address on the stack is the actual address rather than the address-1
        let return_status = self.stack_pop(); // Restore status flags first
        let return_address = self.stack_pop_u16(); // Restore PC

        let _ = self.bus_read(self.program_counter); // dummy read
        self.set_program_counter(return_address);

        let mut restored_flags = Flags::from_bits_truncate(return_status);
        restored_flags.set(Flags::BREAK, false); // BRK flag is always cleared after RTI
        restored_flags.set(Flags::BREAK2, true); // BRK2 flag is always cleared after RTI
        self.status = restored_flags;

        // Pop the most recent interrupt type
        self.interrupt_stack.pop();
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
        let value = self.bus_read(address);
        let result = value & self.register_a;
        self.status.set(Flags::ZERO, result == 0);

        // These flags are set based on bits from the original fetched data
        self.status.set(Flags::NEGATIVE, value & (1 << 7) != 0);
        self.status.set(Flags::OVERFLOW, value & (1 << 6) != 0);
    }

    fn slo(&mut self, opcode: &opcodes::Opcode) {
        let shifted_result = self.asl(opcode);
        let ora_result = self.register_a | shifted_result;
        self.set_register_a(ora_result);
        self.update_zero_and_negative_flags(ora_result);
    }

    fn nop_page_cross(&mut self, opcode: &opcodes::Opcode) {
        let (_address, boundary_cross) = self.get_parameter_address(&opcode.mode);
        self.extra_cycles += boundary_cross as u8;
    }

    /////////////////////////
    /// Illegal Opcodes
    /////////////////////////

    fn sax(&mut self, opcode: &opcodes::Opcode) {
        // SAX => A AND X -> M
        /* A and X are put on the bus at the same time (resulting effectively
          in an AND operation) and stored in M
        */
        let (address, _) = self.get_parameter_address(&opcode.mode);
        let result = self.register_a & self.register_x;
        self.bus_write(address, result);
    }

    fn sbx(&mut self, opcode: &opcodes::Opcode) {
        let (address, _) = self.get_parameter_address(&opcode.mode);
        let value = self.bus_read(address);

        let and_result = self.register_a & self.register_x;
        let result = and_result.wrapping_sub(value);

        self.register_x = result;
        self.status.set(Flags::CARRY, and_result >= value);
        self.update_zero_and_negative_flags(result);
    }

    fn anc(&mut self, opcode: &opcodes::Opcode) {
        let (address, _) = self.get_parameter_address(&opcode.mode);
        let value = self.bus_read(address);
        self.set_register_a(self.register_a & value);
        self.status
            .set(Flags::CARRY, self.register_a & 0b1000_0000 != 0);
    }

    fn arr(&mut self, opcode: &opcodes::Opcode) {
        // ARR => AND + ROR with special flag behavior
        let (address, _) = self.get_parameter_address(&opcode.mode);
        let value = self.bus_read(address);
        self.register_a &= value;

        // Perform ROR (Rotate Right) with Carry
        let carry = self.status.contains(Flags::CARRY) as u8;
        let result = (self.register_a >> 1) | (carry << 7);
        self.register_a = result;
        self.update_zero_and_negative_flags(result);

        // Set Carry flag based on bit 6
        self.status.set(Flags::CARRY, result & 0b0100_0000 != 0);

        // Set Overflow flag based on bits 6 and 5
        let bit6 = result & 0b0100_0000 != 0;
        let bit5 = result & 0b0010_0000 != 0;
        self.status.set(Flags::OVERFLOW, bit6 ^ bit5);
    }

    fn brk(&mut self) {
        let _ = self.bus_read(self.program_counter); // dummy read

        // BRK - Software-defined Interrupt
        self.program_counter = self.program_counter.wrapping_add(1); // BRK has an implied operand, so increment PC before pushing
        self.handle_interrupt(interrupts::BRK);
    }

    fn sre(&mut self, opcode: &opcodes::Opcode) {
        // SRE => LSR oper + EOR oper
        let result = self.lsr(opcode); // LSR
        self.set_register_a(self.register_a ^ result); // A ^ M -> A
        self.extra_cycles = 0;
    }

    fn rla(&mut self, opcode: &opcodes::Opcode) {
        // RLA => ROL oper + AND oper
        let result = self.rol(opcode); // ROL
        self.set_register_a(self.register_a & result); // M & A -> A
        self.extra_cycles = 0;
    }

    fn las(&mut self, opcode: &opcodes::Opcode) {
        // LAS (LAR) => AND with SP, store in A, X, SP
        let (address, boundary_crossed) = self.get_parameter_address(&opcode.mode);
        let value = self.bus_read(address);
        self.set_register_a(value);

        // Perform AND operation with the stack pointer
        let result = value & self.stack_pointer;
        self.register_a = result;
        self.register_x = result;
        self.stack_pointer = result;

        self.update_zero_and_negative_flags(result);
        self.extra_cycles += boundary_crossed as u8;
    }
}

fn is_boundary_crossed(addr1: u16, addr2: u16) -> bool {
    addr1 & 0xFF00 != addr2 & 0xFF00
}

pub fn rotate_value_left(value: u8, current_carry: bool) -> (u8, bool) {
    let new_carry = value & 0b1000_0000 != 0;
    let mut shifted = value << 1;
    shifted |= current_carry as u8;
    (shifted, new_carry)
}

pub fn rotate_value_right(value: u8, current_carry: bool) -> (u8, bool) {
    let new_carry = value & 0b0000_0001 != 0;
    let mut shifted = value >> 1;
    shifted |= (current_carry as u8) << 7;
    (shifted, new_carry)
}
