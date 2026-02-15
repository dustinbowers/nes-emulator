use super::super::trace;
use super::opcodes::Opcode;
use super::{CPU, CpuCycleState, CpuError, Flags, interrupts, opcodes};
use std::collections::HashMap;

impl CPU {
    #[allow(dead_code)]
    pub fn run(&mut self) {
        loop {
            let (_instr_done, is_breaking) = self.tick(true);
            if is_breaking {
                break;
            }
        }
    }

    pub(super) fn advance_program_counter(&mut self) {
        self.program_counter = self.program_counter.wrapping_add(1);
    }

    // pub(super) fn read_program_counter(&mut self) -> u8 {
    //     self.bus_read(self.program_counter)
    // }
    pub(super) fn try_read_program_counter(&mut self) -> Option<u8> {
        self.try_bus_read(self.program_counter)
    }

    pub(super) fn try_consume_program_counter(&mut self) -> Option<u8> {
        let byte = self.try_read_program_counter()?;
        self.advance_program_counter();
        Some(byte)
    }

    // pub(super) fn consume_program_counter(&mut self) -> u8 {
    //     let byte = self.read_program_counter();
    //     if self.stalled_this_tick {
    //         return 0;
    //     }
    //     self.advance_program_counter();
    //     byte
    // }

    /// Runs one CPU cycle
    ///
    /// # Returns
    ///
    /// A tuple `(bool, bool, bool)`:
    /// - The first element is `true` if the CPU stalled during the tick
    /// - The second element is `true` if CPU this cycle completed an instruction
    /// - The third element is `true` if CPU is breaking (due to JAM/KIL instruction)
    // pub fn tick(&mut self, rdy_line: bool) -> (bool, bool, bool) {
    //     self.rdy_line = rdy_line;
    //     self.stalled_this_tick = false;
    //
    //     // snapshot to revert to if CPU stalls on read
    //     let saved_pc = self.program_counter;
    //     let saved_op = self.current_op.clone();
    //     let saved_active_interrupt = self.active_interrupt;
    //
    //     let (done, is_breaking) = self.tick_inner();
    //
    //     // rollback last tick when rdy_line is low and CPU hits a read cycle
    //     if self.stalled_this_tick {
    //         self.program_counter = saved_pc;
    //         self.current_op = saved_op;
    //         self.active_interrupt = saved_active_interrupt;
    //         return (true, false, false);
    //     }
    //
    //     (false, done, is_breaking)
    // }

    /// Runs one CPU cycle
    ///
    /// # Returns
    ///
    /// A tuple `(bool, bool)`:
    /// * The first element is true if current instruction is complete
    /// * The second element is true if CPU is breaking (due to JAM/KIL instruction)
    pub fn tick(&mut self, rdy_line: bool) -> (bool, bool) {
        self.rdy_line = rdy_line;
        self.stalled_this_tick = false;

        self.cycle = (self.cycle + 1) % 3_000_000;

        // Load next opcode if empty
        if self.current_op.opcode.is_none() {
            // We're at instruction boundary with no active interrupts, so check for pending NMI
            if self.active_interrupt.is_none() {
                let curr_nmi_line = self.nmi_line();
                if !curr_nmi_line {
                    self.nmi_armed = true;
                }

                if self.nmi_enable_holdoff > 0 {
                    self.nmi_enable_holdoff -= 1;
                } else if curr_nmi_line && self.nmi_armed {
                    self.nmi_armed = false;
                    self.active_interrupt = Some(interrupts::NMI);

                    #[cfg(feature = "tracing")]
                    {
                        let (scanline, dot) = unsafe { (*self.bus.unwrap()).ppu_timing() };
                        trace_cpu_event!("[NMI SET] sl={} dot={}", scanline, dot);
                    }
                }
            }

            // Still no interrupt, now check for IRQs
            if self.active_interrupt.is_none() {
                // Handle IRQ
                let irq_line = unsafe { (*self.bus.unwrap()).irq_line() };
                if irq_line && !self.status.contains(Flags::INTERRUPT_DISABLE) {
                    self.active_interrupt = Some(interrupts::IRQ);
                }
            }

            // Handle Interrupt if one is waiting
            if let Some(interrupt) = self.active_interrupt {
                let done = self.exec_interrupt_cycle(interrupt);
                if done {
                    self.active_interrupt = None;
                }
                return (done, false);
            }

            // Load next opcode
            let opcodes: &HashMap<u8, &'static Opcode> = &opcodes::OPCODES_MAP;
            let code = match self.try_consume_program_counter() {
                Some(v) => v,
                None => return (false, false),
            };

            let opcode = match opcodes.get(&code).copied() {
                Some(op) => op,
                None => {
                    self.error = Some(CpuError::UnknownOpcode(code));
                    return (true, true);
                }
            };
            self.current_op = CpuCycleState::default();
            self.last_opcode_desc = opcode.name.parse().unwrap();
            self.current_op.opcode = Some(opcode);
            self.current_op.access_type = opcode.access_type;

            // NOTE: I've assigned the 0x02 opcode (normally a JAM/KIL) to break out of the CPU run loop for testing purposes
            let is_breaking: bool = if opcode.code == 0x02 {
                self.error = Some(CpuError::JamOpcode(opcode.code));
                self.set_program_counter(self.program_counter - 1); // Loop on the same opcode
                true
            } else {
                false
            };
            return (false, is_breaking);
        }

        // Execute current instruction
        let opcode = self.current_op.opcode.unwrap();
        let done = (opcode.exec)(self);

        // Prepare for next opcode
        if done {
            let (sl, dot) = unsafe { (*self.bus.unwrap()).ppu_timing() };
            if (sl == 240 && dot > 330) || (sl == 241 && dot < 10) {
                trace!(
                    "[CPU INSN DONE] PC={:04X} opcode=0x{:02X} name={} cycle={} PPU=({},{})",
                    self.program_counter, opcode.code, self.last_opcode_desc, self.cycle, sl, dot
                );
            }
            self.current_op = CpuCycleState::default();
        }

        (done, false)
    }
}
