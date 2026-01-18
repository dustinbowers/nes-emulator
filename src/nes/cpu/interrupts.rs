// See: https://www.nesdev.org/wiki/CPU_interrupts#IRQ_and_NMI_tick-by-tick_execution

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum InterruptType {
    Nmi, // Non-maskable interrupt (triggered from PPU at VBLANK)
    Irq,
    Brk, // Software-defined interrupt
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Interrupt {
    pub interrupt_type: InterruptType,
    pub vector_addr: u16,
    pub b_flag_mask: u8,
    pub cpu_cycles: u8,
}

pub const NMI: Interrupt = Interrupt {
    interrupt_type: InterruptType::Nmi,
    vector_addr: 0xFFFA, // NMI address vector lives at $FFFA
    b_flag_mask: 0b0010_0000,
    cpu_cycles: 7,
};

pub const BRK: Interrupt = Interrupt {
    interrupt_type: InterruptType::Brk,
    vector_addr: 0xFFFE,      // brk address vector lives at $FFFE
    b_flag_mask: 0b0011_0000,
    cpu_cycles: 7,
};

pub const IRQ: Interrupt = Interrupt {
    interrupt_type: InterruptType::Irq,
    vector_addr: 0xFFFE, // IRQ address vector lives at $FFFE
    b_flag_mask: 0b0000_0000,
    cpu_cycles: 7,
};
