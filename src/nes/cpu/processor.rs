use bitflags::bitflags;
use std::collections::HashMap;
use thiserror::Error;

use super::super::trace;
use super::opcodes::Opcode;
use super::interrupts::{Interrupt, InterruptType};
use super::{interrupts, opcodes, CpuBusInterface, CpuCycleState, CpuError, CpuMode, Flags, CPU, CPU_STACK_RESET, DEBUG};

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
            program_counter: 0,
            current_op: CpuCycleState::default(),
            nmi_pending: false,
            interrupt_stack: vec![],
            last_opcode_desc: "".to_string(),
            error: None,
            stop: false,
        };
        cpu
    }

    pub fn reset(&mut self) {
        self.cycles = 0;
        self.halt_scheduled = false;
        self.rdy = true;
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.stack_pointer = CPU_STACK_RESET;
        self.status = Flags::from_bits_truncate(0b0010_0010);
        self.program_counter = self.bus_read_u16(0xFFFC);
        self.current_op = CpuCycleState::default();
        self.nmi_pending = false; // PPU will notify CPU when NMI needs handling
        self.interrupt_stack = vec![]; // This prevents nested NMI (while allowing nested BRKs)
        self.last_opcode_desc = "".to_string();
        self.error = None;
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

    // #[allow(dead_code)]
    // pub fn run(&mut self) {
    //     loop {
    //         let (_, _, should_break) = self.tick();
    //         if should_break {
    //             break;
    //         }
    //     }
    // }

    pub fn trigger_nmi(&mut self) {
        self.nmi_pending = true;
    }

    fn toggle_mode(&mut self) {
        self.cpu_mode = match &self.cpu_mode {
            CpuMode::Read => CpuMode::Write,
            CpuMode::Write => CpuMode::Read,
        };
    }

    pub(super) fn advance_program_counter(&mut self) {
        self.program_counter = self.program_counter.wrapping_add(1);
    }

    pub(super) fn read_program_counter(&self) -> u8{
        self.bus_read(self.program_counter)
    }

    pub(super) fn consume_program_counter(&mut self) -> u8 {
        let byte = self.read_program_counter();
        self.advance_program_counter();
        byte
    }

    // FIXME: This is just an empty stub for refactoring


    pub fn tick(&mut self) -> (bool, bool) {
        // TODO: Handle DMA
        // TODO: Handle Interrupts

        // Load next opcode if empty
        if self.current_op.opcode.is_none() {
            println!("===========================================\n=== loading next opcode... \n===========================================");
            let opcodes: &HashMap<u8, &'static Opcode> = &opcodes::OPCODES_MAP;
            let code = self.consume_program_counter();

            let opcode = match opcodes.get(&code).copied() {
                Some(op) => op,
                None => {
                    self.error = Some(CpuError::UnknownOpcode(code));
                    return (true, true);
                }
            };
            self.current_op = CpuCycleState::default();
            self.current_op.opcode = Some(opcode);
            self.current_op.access_type = opcode.access_type;
            dbg!(&self.current_op);
            return (false, false)
        }

        // Execute current instruction
        let opcode = self.current_op.opcode.unwrap();
        println!("START executing opcode (PC = {:04X})", self.program_counter);
        let done = (opcode.exec)(self);
        println!("DONE executing opcode (PC = {:04X}), done = {}", self.program_counter, done);
        dbg!(&self.current_op);

        // Prepare for next opcode
        if done {
            dbg!("opcode DONE");
            self.current_op = CpuCycleState::default();
        }

        println!("cycles:{}, current_op:{:?}", self.cycles, self.current_op);

        (done, false)
    }

    // // `tick` returns (num_cycles, bytes_consumed, is_breaking)
    // pub fn tick(&mut self) -> (u8, u8, bool) {
    //     // Stall for previous cycles from last instruction
    //     if self.skip_cycles > 0 {
    //         self.skip_cycles -= 1;
    //         self.toggle_mode();
    //         self.cycles += 1;
    //         return (0, 0, false);
    //     }
    //
    //     // DMAs schedule halts, which triggers a set of events:
    //     // - CPU finishes current instruction (above)
    //     // - CPU waits for "Read" state
    //     // - CPU halts for 1 cycle to enter DMA mode
    //     if self.halt_scheduled {
    //         match self.cpu_mode {
    //             CpuMode::Read => {
    //                 self.rdy = false;
    //                 self.halt_scheduled = false;
    //             }
    //             CpuMode::Write => {
    //                 trace!("OAM DMA DUMMY READ");
    //                 self.toggle_mode();
    //             }
    //         }
    //         self.cycles += 1;
    //         return (0, 0, false);
    //     }
    //
    //     self.toggle_mode();
    //
    //     // If we're not already handling NMI, immediately handle it
    //     if !self.interrupt_stack.contains(&InterruptType::Nmi) && self.nmi_pending {
    //         self.nmi_pending = false;
    //         self.handle_interrupt(interrupts::NMI);
    //     }
    //
    //     let ref opcodes: HashMap<u8, &'static opcodes::Opcode> = *opcodes::OPCODES_MAP;
    //
    //     self.extra_cycles = 0;
    //     self.skip_pc_advance = false;
    //     let code = self.bus_read(self.program_counter);
    //     let opcode_lookup = opcodes.get(&code);
    //     let opcode = match opcode_lookup {
    //         Some(opcode) => *opcode,
    //         None => {
    //             // self.tracer.print_trace();
    //             self.error = Some(CpuError::UnknownOpcode(code));
    //             return (0, 0, true);
    //         }
    //     };
    //
    //     {
    //         // Build debug trace
    //         let mut operand_bytes: Vec<u8> = vec![];
    //         for i in 1..opcode.size {
    //             let address = self.program_counter.wrapping_add(i as u16);
    //             operand_bytes.push(self.bus_read(address));
    //         }
    //         let trace = format!(
    //             "({}) PC:${:04X} SP:${:02X} A:${:02X} X:${:02X} Y:${:02X} status: 0b{:08b} \tOpcode: (${:02X}) {} {:02X?}",
    //             self.program_counter,
    //             self.program_counter,
    //             self.stack_pointer,
    //             self.register_a,
    //             self.register_x,
    //             self.register_y,
    //             self.status.bits(),
    //             self.bus_read(self.program_counter),
    //             opcode.name,
    //             operand_bytes
    //         );
    //         self.last_opcode_desc = format!("Opcode: {} {:02x?}", opcode.name, operand_bytes);
    //         if DEBUG {
    //             println!("{trace}");
    //         }
    //         // trace!("{}", format!("CPU: {}", trace));
    //         // self.tracer.write(trace);
    //     }
    //
    //     self.program_counter = self.program_counter.wrapping_add(1);
    //
    //     match code {
    //         0x00 => self.brk(), // BRK
    //         0xEA => {}          // NOP
    //
    //         0x4C => self.jmp(opcode), // JMP Absolute
    //         0x6C => self.jmp(opcode), // JMP Indirect (with 6502 bug)
    //         0x20 => self.jsr(opcode), // JSR
    //         0x60 => self.rts(),       // RTS
    //         0x40 => self.rti(),       // RTI
    //
    //         0xAA => self.tax(), // TAX
    //         0xA8 => self.tay(), // TAY
    //         0xBA => self.tsx(), // TSX
    //         0x8A => self.txa(), // TXA
    //         0x9A => self.txs(), // TXS
    //         0x98 => self.tya(), // TYA
    //
    //         0xD8 => self.cld(), // CLD
    //         0x58 => self.cli(), // CLI
    //         0xB8 => self.clv(), // CLV
    //         0x18 => self.clc(), // CLC
    //         0x38 => self.sec(), // SEC
    //         0x78 => self.sei(), // SEI
    //         0xF8 => self.sed(), // SED
    //
    //         0xD0 => self.bne(opcode), // BNE
    //         0x70 => self.bvs(opcode), // BVS
    //         0x50 => self.bvc(opcode), // BVC
    //         0x30 => self.bmi(opcode), // BMI
    //         0xF0 => self.beq(opcode), // BEQ
    //         0xB0 => self.bcs(opcode), // BCS
    //         0x90 => self.bcc(opcode), // BCC
    //         0x10 => self.bpl(opcode), // BPL
    //
    //         0xE8 => self.inx(), // INX
    //         0xC8 => self.iny(), // INY
    //
    //         0xCA => self.dex(), // DEX
    //         0x88 => self.dey(), // DEY
    //
    //         0x48 => self.pha(), // PHA
    //         0x68 => self.pla(), // PLA
    //         0x08 => self.php(), // PHP
    //         0x28 => self.plp(), // PLP
    //
    //         0x24 | 0x2C => self.bit(opcode), // BIT
    //
    //         0xA9 | 0xA5 | 0xB5 | 0xAD | 0xBD | 0xB9 | 0xA1 | 0xB1 => {
    //             self.lda(opcode); // LDA
    //         }
    //         0xA2 | 0xA6 | 0xB6 | 0xAE | 0xBE => {
    //             self.ldx(opcode); // LDX
    //         }
    //         0xA0 | 0xA4 | 0xB4 | 0xAC | 0xBC => {
    //             self.ldy(opcode); // LDY
    //         }
    //         0x85 | 0x95 | 0x8D | 0x9D | 0x99 | 0x81 | 0x91 => {
    //             self.sta(opcode); // STA
    //         }
    //         0x86 | 0x96 | 0x8e => {
    //             self.stx(opcode); // STX
    //         }
    //         0x84 | 0x94 | 0x8c => {
    //             self.sty(opcode); // STY
    //         }
    //         0x0A | 0x06 | 0x16 | 0x0E | 0x1E => {
    //             self.asl(opcode); // ASL
    //         }
    //         0x4A | 0x46 | 0x56 | 0x4E | 0x5E => {
    //             self.lsr(opcode); // LSR
    //         }
    //         0x2A | 0x26 | 0x36 | 0x2E | 0x3E => {
    //             self.rol(opcode); // ROL
    //         }
    //         0x6A | 0x66 | 0x76 | 0x6E | 0x7E => {
    //             self.ror(opcode); // ROR
    //         }
    //         0xE6 | 0xF6 | 0xEE | 0xFE => {
    //             self.inc(opcode); // INC
    //         }
    //         0xC6 | 0xD6 | 0xCE | 0xDE => {
    //             self.dec(opcode); // DEC
    //         }
    //         0xC9 | 0xC5 | 0xD5 | 0xCD | 0xDD | 0xD9 | 0xC1 | 0xD1 => {
    //             self.cmp(opcode); // CMP
    //         }
    //         0xE0 | 0xE4 | 0xEC => {
    //             self.cpx(opcode); // CPX
    //         }
    //         0xC0 | 0xC4 | 0xCC => {
    //             self.cpy(opcode); // CPY
    //         }
    //         0x69 | 0x65 | 0x75 | 0x6D | 0x7D | 0x79 | 0x61 | 0x71 => {
    //             self.adc(opcode); // ADC
    //         }
    //         0xE9 | 0xE5 | 0xF5 | 0xED | 0xFD | 0xF9 | 0xE1 | 0xF1 => {
    //             self.sbc(opcode); // SBC
    //         }
    //         0x29 | 0x25 | 0x35 | 0x2D | 0x3D | 0x39 | 0x21 | 0x31 => {
    //             self.and(opcode); // AND
    //         }
    //         0x49 | 0x45 | 0x55 | 0x4D | 0x5D | 0x59 | 0x41 | 0x51 => {
    //             self.eor(opcode); // EOR
    //         }
    //         0x09 | 0x05 | 0x15 | 0x0D | 0x1D | 0x19 | 0x01 | 0x11 => {
    //             self.ora(opcode); // ORA
    //         }
    //
    //         /////////////////////////
    //         // Illegal Opcodes
    //         /////////////////////////
    //         0xC7 | 0xD7 | 0xCF | 0xDF | 0xDB | 0xD3 | 0xC3 => {
    //             // DCP => DEC oper + CMP oper
    //             self.dcp(opcode);
    //         }
    //         0x27 | 0x37 | 0x2F | 0x3F | 0x3B | 0x33 | 0x23 => {
    //             // RLA => ROL oper + AND oper
    //             self.rla(opcode);
    //         }
    //         0x07 | 0x17 | 0x0F | 0x1F | 0x1B | 0x03 | 0x13 => {
    //             // SLO => ASL oper + ORA oper
    //             self.slo(opcode);
    //         }
    //         0x47 | 0x57 | 0x4F | 0x5F | 0x5B | 0x43 | 0x53 => {
    //             // SRE => LSR oper + EOR oper
    //             self.sre(opcode);
    //         }
    //         0x67 | 0x77 | 0x6F | 0x7F | 0x7B | 0x63 | 0x73 => {
    //             // FIXME: Hand-roll this instead of chaining 2 instructions
    //             // RRA => ROR oper + ADC oper
    //             self.ror(opcode);
    //             self.adc(opcode);
    //             self.extra_cycles = 0;
    //         }
    //         0xE7 | 0xF7 | 0xEF | 0xFF | 0xFB | 0xE3 | 0xF3 => {
    //             // ISC (ISB / INS) => INC oper + SBC oper
    //             self.isc(opcode);
    //         }
    //         0xA7 | 0xB7 | 0xAF | 0xBF | 0xA3 | 0xB3 => {
    //             // FIXME: Hand-roll this instead of chaining 2 instructions
    //             // LAX => LDA oper + LDX oper
    //             self.lda(opcode);
    //             self.ldx(opcode);
    //             if self.extra_cycles == 2 {
    //                 self.extra_cycles = 1;
    //             }
    //         }
    //         0x87 | 0x97 | 0x8F | 0x83 => {
    //             // SAX => A AND X -> M
    //             self.sax(opcode);
    //         }
    //         0xCB => {
    //             // SBX => CMP and DEX at once, sets flags like CMP
    //             self.sbx(opcode);
    //         }
    //         0x6B => {
    //             // ARR => AND oper + ROR (Plus some wonky flag manipulation)
    //             self.arr(opcode);
    //         }
    //         0xEB => {
    //             // USBC (SBC) => SBC oper + NOP
    //             self.sbc(opcode);
    //         }
    //         0x0B | 0x2B => {
    //             // ANC => A AND oper, bit(7) -> C
    //             self.anc(opcode);
    //         }
    //         0x4B => {
    //             // FIXME: Hand-roll this instead of chaining 2 instructions
    //             // ALR => AND oper + LSR
    //             self.and(opcode);
    //             self.lsr(opcode);
    //         }
    //         0xBB => {
    //             // LAS (LAR) => LDA + AND with SP, store in A, X, SP
    //             self.las(opcode);
    //         }
    //         0x02 => {
    //             // JAM - This freezes the CPU
    //             // NOTE: I'm hijacking this opcode for use in processor_tests
    //             //       0x02 now breaks the normal run() loop{}
    //             return if cfg!(test) {
    //                 self.cycles += 1;
    //                 (11, 1, true)
    //             } else {
    //                 self.error = Some(CpuError::JamOpcode(opcode.value));
    //                 (0, 0, true)
    //             }
    //         }
    //         0x12 | 0x22 | 0x32 | 0x42 | 0x52 | 0x62 | 0x72 | 0x92 | 0xB2 | 0xD2 | 0xF2 => {
    //             // JAM - These instructions freeze the CPU
    //             self.error = Some(CpuError::JamOpcode(opcode.value));
    //             return (0, 0, true);
    //         }
    //
    //         0x8B | 0xAB | 0x9F | 0x93 | 0x9E | 0x9C | 0x9B => {
    //             // Unstable and highly-unstable opcodes (Purposely unimplemented)
    //             self.error = Some(CpuError::UnstableOpcode(opcode.value));
    //             return (0, 0, true);
    //         }
    //
    //         0x1C | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC => {
    //             // page-crossing NOPs
    //             self.nop_page_cross(opcode);
    //         }
    //
    //         0x80 | 0x82 | 0x89 | 0xC2 | 0xE2 | 0x04 | 0x44 | 0x64 | 0x14 | 0x34 | 0x54 | 0x74
    //         | 0xD4 | 0xF4 | 0x0C | 0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xFA => {
    //             // Various single and multiple-byte NOPs
    //         }
    //         // _ => unreachable!(),
    //     }
    //
    //     // Tick the bus for opcode cycles. Add any extra cycles from boundary_crosses and other special cases
    //     let cycle_count = opcode.cycles + self.extra_cycles;
    //
    //     // Subtract one to account for execution of the current cycle
    //     self.skip_cycles = cycle_count - 1;
    //
    //     // Advance PC normally if an opcode didn't modify it
    //     if !self.skip_pc_advance {
    //         self.program_counter = self.program_counter.wrapping_add((opcode.size - 1) as u16);
    //     }
    //     self.cycles += 1;
    //     (cycle_count, opcode.size, false)
    // }
}

