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
    pub fn new() -> CPU {
        CPU {
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
            active_interrupt: None,
            nmi_pending: false,
            irq_pending: false,
            last_opcode_desc: "".to_string(),
            error: None,
            stop: false,
        }
    }

    pub fn reset(&mut self) {
        let pcl = self.bus_read(0xFFFC) as u16;
        let pch = self.bus_read(0xFFFD) as u16;
        self.program_counter = (pch << 8) | pcl;
        self.current_op = CpuCycleState::default();
        self.last_opcode_desc = "".to_string();
        self.cycles = 0;
        self.halt_scheduled = false;
        self.rdy = true;
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.stack_pointer = CPU_STACK_RESET;
        self.status = Flags::from_bits_truncate(0b0010_0010);
        self.active_interrupt = None;
        self.nmi_pending = false; // PPU will notify CPU when NMI needs handling
        self.irq_pending = false;
        self.error = None;
    }

    /// `connect_bus` MUST be called after constructing CPU
    pub fn connect_bus(&mut self, bus: *mut dyn CpuBusInterface) {
        self.bus = Some(bus);
        let pcl = self.bus_read(0xFFFC) as u16;
        let pch = self.bus_read(0xFFFD) as u16;
        self.program_counter = (pch << 8) | pcl;
    }

    /// `bus_read` is safe because Bus owns CPU
    pub fn bus_read(&self, addr: u16) -> u8 {
        unsafe { (*self.bus.unwrap()).cpu_bus_read(addr) }
    }

    /// `bus_write` is safe because Bus owns CPU
    pub fn bus_write(&self, addr: u16, data: u8) {
        unsafe {
            (*self.bus.unwrap()).cpu_bus_write(addr, data);
        }
    }

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

    pub fn tick(&mut self) -> (bool, bool) {
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
                        trace!("Pausing CPU for DMA");
                        self.rdy = false; // This pauses CPU execution while DMA runs
                        self.halt_scheduled = false;
                    }
                    CpuMode::Write => {
                        trace!("OAM DMA DUMMY READ");
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
                trace!("[INTERRUPT] {:?}", interrupt);
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

            // trace_obj!(&*self);
            return (false, false);
        }

        // Execute current instruction
        let opcode = self.current_op.opcode.unwrap();
        let done = (opcode.exec)(self);
        // trace_obj!(&*self);

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
