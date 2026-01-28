use super::super::trace;
use super::interrupts::{Interrupt, InterruptType};
use super::opcodes::Opcode;
use super::{
    CPU, CPU_STACK_RESET, CpuBusInterface, CpuCycleState, CpuError, CpuMode, DEBUG, Flags,
    interrupts, opcodes,
};
use crate::trace_obj;
use bitflags::bitflags;
use std::collections::HashMap;
use thiserror::Error;

impl CPU {
    #[allow(dead_code)]
    pub fn run(&mut self) {
        loop {
            let (_, should_break) = self.tick();
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

    pub(super) fn advance_program_counter(&mut self) {
        self.program_counter = self.program_counter.wrapping_add(1);
    }

    pub(super) fn read_program_counter(&self) -> u8 {
        self.bus_read(self.program_counter)
    }

    pub(super) fn consume_program_counter(&mut self) -> u8 {
        let byte = self.read_program_counter();
        self.advance_program_counter();
        byte
    }

    /// Runs one CPU cycle
    ///
    /// # Returns
    ///
    /// A tuple `(bool, bool)`:
    /// * The first element is true if current instruction is complete
    /// * The second element is true if CPU is breaking (due to JAM/KIL instruction)
    pub fn tick(&mut self) -> (bool, bool) {
        self.cycle += 1;
        if self.cycle >= 1_000_000 {
            self.cycle -= 1_000_000; // Prevent overflow
        }

        if self.nmi_pending {
            self.nmi_pending = false;
            self.active_interrupt = Some(interrupts::NMI);
        } else if self.irq_pending && !self.status.contains(Flags::INTERRUPT_DISABLE) {
            self.irq_pending = false;
            self.active_interrupt = Some(interrupts::IRQ);
        }

        // Load next opcode if empty
        if self.current_op.opcode.is_none() {
            // DMAs schedule halts, which triggers a set of events:
            // - CPU waits for "Read" state
            // - CPU halts for 1 cycle to enter DMA mode
            if self.halt_scheduled {
                match self.cpu_mode {
                    CpuMode::Read => {
                        self.rdy = false; // This pauses CPU execution while DMA runs
                        self.halt_scheduled = false;
                    }
                    CpuMode::Write => {
                        self.toggle_mode();
                    }
                }
                return (false, false);
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
            let code = self.consume_program_counter();

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
            self.current_op = CpuCycleState::default();
        }

        (done, false)
    }

    fn start_interrupt(&mut self, interrupt: Interrupt) {
        self.current_op.interrupt = Some(interrupt);
    }
}
