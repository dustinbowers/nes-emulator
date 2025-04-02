// See: https://www.nesdev.org/wiki/CPU_interrupts#IRQ_and_NMI_tick-by-tick_execution

#[derive(PartialEq, Eq)]
pub enum InterruptType {
    NMI, // Non-maskable interrupt (triggered from PPU at VBLANK)
    BRK, // Software-defined interrupt
}

#[derive(PartialEq, Eq)]
pub struct Interrupt {
    pub interrupt_type: InterruptType,
    pub vector_addr: u16,
    pub b_flag_mask: u8,
    pub cpu_cycles: u8,
}

pub const NMI: Interrupt = Interrupt {
    interrupt_type: InterruptType::NMI,
    vector_addr: 0xFFFA, // NMI address vector lives at $FFFA
    b_flag_mask: 0b0010_0000,
    cpu_cycles: 2,
};

pub const BRK: Interrupt = Interrupt {
    interrupt_type: InterruptType::BRK,
    vector_addr: 0xFFFE, // BRK address vector lives at $FFFE
    b_flag_mask: 0b0011_0000, // TODO: i think this is supposed to be 0b0010_0000
    cpu_cycles: 0,
};
