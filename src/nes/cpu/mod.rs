use bitflags::bitflags;
use thiserror::Error;
use interrupts::InterruptType;
use opcodes::{Opcode};
use crate::nes::cpu::interrupts::Interrupt;
use super::tracer::Traceable;

mod interrupts;
mod opcodes;
mod processor;
mod instruction_handlers;

#[cfg(test)]
pub mod processor_tests;
mod trace;

// const DEBUG: bool = true;
const DEBUG: bool = false;
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

#[derive(Debug, PartialEq, Clone, Copy)]
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

#[derive(Default, Debug, PartialEq, Clone, Copy)]
pub enum AccessType {
    None,
    #[default]
    Read,
    Write,
    ReadModifyWrite,
    Register,
}

#[derive(Debug)]
pub enum CpuMode {
    Read,
    Write,
}

#[derive(Debug, Error)]
pub enum CpuError {
    #[error("JAM opcode encountered: 0x{0:02X}")]
    JamOpcode(u8),

    #[error("Unknown opcode: 0x{0:02X}")]
    UnknownOpcode(u8),

    #[error("Unstable opcode: 0x{0:02X}")]
    UnstableOpcode(u8),

    #[error("Invalid NMI encountered")]
    InvalidNMI,

    #[error("Invalid opcode: 0x{0:02X}")]
    InvalidOpcode(u8),
}

#[derive(Default, Debug, Clone, Copy)]
enum AddrResult {
    #[default]
    InProgress,
    Ready(u16),
    ReadyImmediate(u8),
}

#[derive(Default, Debug, Copy, Clone, PartialEq)]
enum ExecPhase {
    #[default]
    Idle,     // instruction waiting for address resolution
    Read,
    Internal, // cpu work (ALU / registers / etc.)
    Write,
    Done,
}

#[derive(Default, Debug)]
struct CpuCycleState {
    opcode: Option<&'static Opcode>,
    interrupt: Option<Interrupt>,
    micro_cycle: u8,
    access_type: AccessType,
    exec_phase: ExecPhase,
    addr_result: AddrResult,

    base_addr: u16,
    tmp_addr: u16,
    tmp_data: u8,
    page_crossed: bool,
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

    current_op: CpuCycleState,

    nmi_pending: bool,
    irq_pending: bool,
    active_interrupt: Option<Interrupt>,

    pub last_opcode_desc: String,
    pub error: Option<CpuError>,
    pub stop: bool,
}


pub trait CpuBusInterface {
    fn cpu_bus_read(&mut self, addr: u16) -> u8;
    fn cpu_bus_write(&mut self, addr: u16, value: u8);
}

impl Traceable for CPU {
    fn trace_name(&self) -> &'static str {
        "CPU"
    }
    fn trace_state(&self) -> Option<String> {
        Some(format!(
            "(micro_cycle: {}, addr_result: {:?}, exec_phase: {:?})\tPC={:04X} A={:02X} X={:02X} Y={:02X} P={:02X} SP={:02X} [{:?}]",
            self.current_op.micro_cycle,
            self.current_op.addr_result,
            self.current_op.exec_phase,
            self.program_counter,
            self.register_a,
            self.register_x,
            self.register_y,
            self.status,
            self.stack_pointer,
            self.last_opcode_desc
        ))
    }
}
