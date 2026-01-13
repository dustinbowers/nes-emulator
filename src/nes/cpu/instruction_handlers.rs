use crate::nes::cpu::interrupts::{Interrupt, InterruptType};
use super::{AddressingMode, Flags, interrupts, opcodes, CPU, CPU_STACK_BASE, CpuError};

impl CPU {
    pub(super) fn lda(&mut self, opcode: &opcodes::Opcode) {
        let (address, boundary_cross) = self.get_parameter_address(&opcode.mode);
        self.extra_cycles += boundary_cross as u8; // boundary_cross adds 1 extra cycle

        let param = self.bus_read(address);
        self.set_register_a(param);
    }

    pub(super) fn ldx(&mut self, opcode: &opcodes::Opcode) {
        let (address, boundary_cross) = self.get_parameter_address(&opcode.mode);
        self.extra_cycles += boundary_cross as u8; // boundary_cross adds 1 extra cycle

        let param = self.bus_read(address);
        self.set_register_x(param);
    }

    pub(super) fn ldy(&mut self, opcode: &opcodes::Opcode) {
        let (address, boundary_cross) = self.get_parameter_address(&opcode.mode);
        self.extra_cycles += boundary_cross as u8; // boundary_cross adds 1 extra cycle

        let param = self.bus_read(address);
        self.set_register_y(param);
    }

    pub(super) fn sta(&mut self, opcode: &opcodes::Opcode) {
        let (address, _) = self.get_parameter_address(&opcode.mode);
        // self.bus_read(address);
        self.bus_write(address, self.register_a);
    }

    pub(super) fn stx(&mut self, opcode: &opcodes::Opcode) {
        let (address, _) = self.get_parameter_address(&opcode.mode);
        self.bus_write(address, self.register_x);
    }

    pub(super) fn sty(&mut self, opcode: &opcodes::Opcode) {
        let (address, _) = self.get_parameter_address(&opcode.mode);
        self.bus_write(address, self.register_y);
    }

    pub(super) fn tax(&mut self) {
        self.set_register_x(self.register_a);
    }

    pub(super) fn tay(&mut self) {
        self.set_register_y(self.register_a);
    }

    pub(super) fn tsx(&mut self) {
        self.set_register_x(self.stack_pointer);
    }

    pub(super) fn txa(&mut self) {
        self.set_register_a(self.register_x);
    }

    pub(super) fn txs(&mut self) {
        self.stack_pointer = self.register_x;
    }

    pub(super) fn tya(&mut self) {
        self.set_register_a(self.register_y);
    }

    pub(super) fn cld(&mut self) {
        self.status.remove(Flags::DECIMAL_MODE);
    }

    pub(super) fn cli(&mut self) {
        self.status.remove(Flags::INTERRUPT_DISABLE);
    }

    pub(super) fn clv(&mut self) {
        self.status.remove(Flags::OVERFLOW);
    }

    pub(super) fn clc(&mut self) {
        self.status.remove(Flags::CARRY);
    }

    pub(super) fn sec(&mut self) {
        self.status.insert(Flags::CARRY);
    }

    pub(super) fn sei(&mut self) {
        self.status.insert(Flags::INTERRUPT_DISABLE);
    }

    pub(super) fn sed(&mut self) {
        self.status.insert(Flags::DECIMAL_MODE);
    }

    pub(super) fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_x);
    }

    pub(super) fn iny(&mut self) {
        self.register_y = self.register_y.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_y);
    }

    pub(super) fn dex(&mut self) {
        self.register_x = self.register_x.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.register_x);
    }

    pub(super) fn dey(&mut self) {
        self.register_y = self.register_y.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.register_y);
    }

    pub(super) fn pha(&mut self) {
        // Push register_a onto the stack
        self.stack_push(self.register_a)
    }

    pub(super) fn pla(&mut self) {
        // Pop stack into register_a
        let value = self.stack_pop();
        self.set_register_a(value);
    }

    pub(super) fn php(&mut self) {
        // Push processor_status onto the stack
        // https://www.nesdev.org/wiki/Status_flags
        // says that B flag is pushed as 1, but not affected on the CPU
        let mut status_copy = Flags::from_bits_truncate(self.status.bits());
        status_copy.insert(Flags::BREAK);
        self.stack_push(status_copy.bits())
    }

    pub(super) fn plp(&mut self) {
        // Pop stack into processor_status
        self.status = Flags::from_bits_truncate(self.stack_pop());
        self.status.remove(Flags::BREAK); // This flag is disabled when fetching
        self.status.insert(Flags::BREAK2); // This flag is supposed to always be 1 on CPU
    }

    pub(super) fn asl(&mut self, opcode: &opcodes::Opcode) -> u8 {
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

    pub(super) fn lsr(&mut self, opcode: &opcodes::Opcode) -> u8 {
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

    pub(super) fn rol(&mut self, opcode: &opcodes::Opcode) -> u8 {
        // Rotate Left through carry flag
        let curr_carry = self.status.contains(Flags::CARRY);
        match opcode.mode {
            AddressingMode::Immediate => {
                let (value, new_carry) = Self::rotate_value_left(self.register_a, curr_carry);
                self.set_register_a(value);
                self.status.set(Flags::CARRY, new_carry);
                value
            }
            _ => {
                let (address, _) = self.get_parameter_address(&opcode.mode);
                let value = self.bus_read(address);
                let (result, new_carry) = Self::rotate_value_left(value, curr_carry);
                self.bus_write(address, result);
                self.update_zero_and_negative_flags(result);
                self.status.set(Flags::CARRY, new_carry);
                result
            }
        }
    }

    pub(super) fn ror(&mut self, opcode: &opcodes::Opcode) {
        // Rotate Right through carry flag
        let curr_carry = self.status.contains(Flags::CARRY);
        match opcode.mode {
            AddressingMode::Immediate => {
                let (value, new_carry) = Self::rotate_value_right(self.register_a, curr_carry);
                self.set_register_a(value);
                self.status.set(Flags::CARRY, new_carry);
            }
            _ => {
                let (address, _) = self.get_parameter_address(&opcode.mode);
                let value = self.bus_read(address);
                let (result, new_carry) = Self::rotate_value_right(value, curr_carry);
                self.bus_write(address, result);
                self.update_zero_and_negative_flags(result);
                self.status.set(Flags::CARRY, new_carry);
            }
        }
    }

    pub(super) fn inc(&mut self, opcode: &opcodes::Opcode) -> u8 {
        // Increment value at Memory
        let (address, _) = self.get_parameter_address(&opcode.mode);
        let mut value = self.bus_read(address);
        value = value.wrapping_add(1);
        self.bus_write(address, value);
        self.update_zero_and_negative_flags(value);
        value
    }

    pub(super) fn dec(&mut self, opcode: &opcodes::Opcode) -> u8 {
        // Decrement value at Memory
        let (address, _) = self.get_parameter_address(&opcode.mode);
        let mut value = self.bus_read(address);
        value = value.wrapping_sub(1);
        self.bus_write(address, value);
        self.update_zero_and_negative_flags(value);
        value
    }

    pub(super) fn cmp(&mut self, opcode: &opcodes::Opcode) {
        // Compare A register
        self.compare(opcode, self.register_a);
    }

    pub(super) fn cpx(&mut self, opcode: &opcodes::Opcode) {
        // Compare X Register
        self.compare(opcode, self.register_x);
    }

    pub(super) fn cpy(&mut self, opcode: &opcodes::Opcode) {
        // Compare Y Register
        self.compare(opcode, self.register_y);
    }

    pub(super) fn adc(&mut self, opcode: &opcodes::Opcode) {
        // Add with Carry
        let (address, boundary_crossed) = self.get_parameter_address(&opcode.mode);
        let value = self.bus_read(address);
        self.add_to_register_a(value);
        self.extra_cycles += boundary_crossed as u8;
    }

    pub(super) fn sbc(&mut self, opcode: &opcodes::Opcode) {
        // Subtract with Carry
        let (address, boundary_crossed) = self.get_parameter_address(&opcode.mode);
        let value = self.bus_read(address);
        self.sub_from_register_a(value);
        self.extra_cycles += boundary_crossed as u8;
    }

    pub(super) fn and(&mut self, opcode: &opcodes::Opcode) {
        // Logical AND on accumulator
        let (address, boundary_crossed) = self.get_parameter_address(&opcode.mode);
        let value = self.bus_read(address);
        self.set_register_a(self.register_a & value);
        self.extra_cycles += boundary_crossed as u8;
    }

    pub(super) fn eor(&mut self, opcode: &opcodes::Opcode) {
        // Logical Exclusive OR on accumulator
        let (address, boundary_crossed) = self.get_parameter_address(&opcode.mode);
        let value = self.bus_read(address);
        self.set_register_a(self.register_a ^ value);
        self.extra_cycles += boundary_crossed as u8;
    }

    pub(super) fn ora(&mut self, opcode: &opcodes::Opcode) {
        // Logical OR on accumulator
        let (address, boundary_crossed) = self.get_parameter_address(&opcode.mode);
        let value = self.bus_read(address);
        self.set_register_a(self.register_a | value);
        self.extra_cycles += boundary_crossed as u8;
    }

    pub(super) fn jmp(&mut self, opcode: &opcodes::Opcode) {
        let (address, _) = self.get_parameter_address(&opcode.mode);
        self.set_program_counter(address);
    }

    pub(super) fn jsr(&mut self, opcode: &opcodes::Opcode) {
        // Jump to Subroutine
        let (jump_address, _) = self.get_parameter_address(&opcode.mode);
        let return_address = self.program_counter.wrapping_add(1);
        self.stack_push_u16(return_address);
        self.set_program_counter(jump_address);
    }

    pub(super) fn rts(&mut self) {
        // Return from Subroutine
        let return_address_minus_one = self.stack_pop_u16();
        let address = return_address_minus_one.wrapping_add(1);

        let _ = self.bus_read(self.program_counter); // dummy read
        self.set_program_counter(address);
    }

    pub(super) fn rti(&mut self) {
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

    pub(super) fn bne(&mut self, opcode: &opcodes::Opcode) {
        // Branch if ZERO is clear
        self.branch(opcode, self.status.contains(Flags::ZERO) == false)
    }

    pub(super) fn bvs(&mut self, opcode: &opcodes::Opcode) {
        // Branch if OVERFLOW is set
        self.branch(opcode, self.status.contains(Flags::OVERFLOW))
    }
    pub(super) fn bvc(&mut self, opcode: &opcodes::Opcode) {
        // Branch if OVERFLOW is clear
        self.branch(opcode, self.status.contains(Flags::OVERFLOW) == false)
    }

    pub(super) fn bmi(&mut self, opcode: &opcodes::Opcode) {
        // Branch if NEGATIVE is set
        self.branch(opcode, self.status.contains(Flags::NEGATIVE))
    }

    pub(super) fn beq(&mut self, opcode: &opcodes::Opcode) {
        // Branch if ZERO is set
        self.branch(opcode, self.status.contains(Flags::ZERO))
    }

    pub(super) fn bcs(&mut self, opcode: &opcodes::Opcode) {
        // Branch if CARRY is set
        self.branch(opcode, self.status.contains(Flags::CARRY))
    }

    pub(super) fn bcc(&mut self, opcode: &opcodes::Opcode) {
        // Branch if CARRY is clear
        self.branch(opcode, self.status.contains(Flags::CARRY) == false)
    }

    pub(super) fn bpl(&mut self, opcode: &opcodes::Opcode) {
        // Branch if NEGATIVE is clear
        self.branch(opcode, self.status.contains(Flags::NEGATIVE) == false)
    }

    pub(super) fn bit(&mut self, opcode: &opcodes::Opcode) {
        // Bit Test
        let (address, _) = self.get_parameter_address(&opcode.mode);
        let value = self.bus_read(address);
        let result = value & self.register_a;
        self.status.set(Flags::ZERO, result == 0);

        // These flags are set based on bits from the original fetched data
        self.status.set(Flags::NEGATIVE, value & (1 << 7) != 0);
        self.status.set(Flags::OVERFLOW, value & (1 << 6) != 0);
    }

    pub(super) fn slo(&mut self, opcode: &opcodes::Opcode) {
        let shifted_result = self.asl(opcode);
        let ora_result = self.register_a | shifted_result;
        self.set_register_a(ora_result);
        self.update_zero_and_negative_flags(ora_result);
    }

    pub(super) fn nop_page_cross(&mut self, opcode: &opcodes::Opcode) {
        let (_address, boundary_cross) = self.get_parameter_address(&opcode.mode);
        self.extra_cycles += boundary_cross as u8;
    }
}

/////////////////////////
// Illegal Opcodes
/////////////////////////
impl CPU {    
    pub(super) fn sax(&mut self, opcode: &opcodes::Opcode) {
        // SAX => A AND X -> M
        /* A and X are put on the bus at the same time (resulting effectively
          in an AND operation) and stored in M
        */
        let (address, _) = self.get_parameter_address(&opcode.mode);
        let result = self.register_a & self.register_x;
        self.bus_write(address, result);
    }

    pub(super) fn sbx(&mut self, opcode: &opcodes::Opcode) {
        let (address, _) = self.get_parameter_address(&opcode.mode);
        let value = self.bus_read(address);

        let and_result = self.register_a & self.register_x;
        let result = and_result.wrapping_sub(value);

        self.register_x = result;
        self.status.set(Flags::CARRY, and_result >= value);
        self.update_zero_and_negative_flags(result);
    }

    pub(super) fn anc(&mut self, opcode: &opcodes::Opcode) {
        let (address, _) = self.get_parameter_address(&opcode.mode);
        let value = self.bus_read(address);
        self.set_register_a(self.register_a & value);
        self.status
            .set(Flags::CARRY, self.register_a & 0b1000_0000 != 0);
    }

    pub(super) fn arr(&mut self, opcode: &opcodes::Opcode) {
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

    pub(super) fn brk(&mut self) {
        let _ = self.bus_read(self.program_counter); // dummy read

        // BRK - Software-defined Interrupt
        self.program_counter = self.program_counter.wrapping_add(1); // BRK has an implied operand, so increment PC before pushing
        self.handle_interrupt(interrupts::BRK);
    }

    pub(super) fn sre(&mut self, opcode: &opcodes::Opcode) {
        // SRE => LSR oper + EOR oper
        let result = self.lsr(opcode); // LSR
        self.set_register_a(self.register_a ^ result); // A ^ M -> A
        self.extra_cycles = 0;
    }

    pub(super) fn rla(&mut self, opcode: &opcodes::Opcode) {
        // RLA => ROL oper + AND oper
        let result = self.rol(opcode); // ROL
        self.set_register_a(self.register_a & result); // M & A -> A
        self.extra_cycles = 0;
    }
    
    pub(super) fn dcp(&mut self, opcode: &opcodes::Opcode) {
        // DCP => DEC oper + CMP oper
        let dec_value = self.dec(opcode);

        // Compare register_a with decremented value
        let result = self.register_a.wrapping_sub(dec_value);
        self.status.set(Flags::CARRY, self.register_a >= dec_value);
        self.update_zero_and_negative_flags(result);
        self.extra_cycles = 0;
    }

    pub(super) fn isc(&mut self, opcode: &opcodes::Opcode) {
        // ISC (ISB / INS) => INC oper + SBC oper
        let inc_result = self.inc(opcode);
        self.sub_from_register_a(inc_result);
    }

    pub(super) fn las(&mut self, opcode: &opcodes::Opcode) {
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

///////////////////////////////////////////////////////////////////////////////
////// Utility functions
///////////////////////////////////////////////////////////////////////////////
impl CPU {
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

                (addr, Self::is_boundary_crossed(base, addr))
            }
            AddressingMode::AbsoluteY => {
                let base = self.bus_read_u16(self.program_counter);
                let addr = base.wrapping_add(self.register_y as u16);

                // Only read from base page (not the final address)
                let dummy_addr =
                    (base & 0xFF00) | ((base.wrapping_add(self.register_y as u16)) & 0x00FF);
                let _ = self.bus_read(dummy_addr);

                (addr, Self::is_boundary_crossed(base, addr))
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
                (addr, Self::is_boundary_crossed(dynamic_base, addr))
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
                let boundary_crossed = Self::is_boundary_crossed(base_pc, target_address);
                (target_address, boundary_crossed)
            }
            _ => unimplemented!(),
        }
    }
    fn is_boundary_crossed(addr1: u16, addr2: u16) -> bool {
        addr1 & 0xFF00 != addr2 & 0xFF00
    }

    pub(super) fn rotate_value_left(value: u8, current_carry: bool) -> (u8, bool) {
        let new_carry = value & 0b1000_0000 != 0;
        let mut shifted = value << 1;
        shifted |= current_carry as u8;
        (shifted, new_carry)
    }

    pub(super) fn rotate_value_right(value: u8, current_carry: bool) -> (u8, bool) {
        let new_carry = value & 0b0000_0001 != 0;
        let mut shifted = value >> 1;
        shifted |= (current_carry as u8) << 7;
        (shifted, new_carry)
    }

    pub(super) fn set_register_a(&mut self, value: u8) {
        self.register_a = value;
        self.update_zero_and_negative_flags(value);
    }

    pub(super) fn set_register_x(&mut self, value: u8) {
        self.register_x = value;
        self.update_zero_and_negative_flags(value);
    }

    pub(super) fn set_register_y(&mut self, value: u8) {
        self.register_y = value;
        self.update_zero_and_negative_flags(value);
    }

    pub(super) fn set_program_counter(&mut self, address: u16) {
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

    pub(super) fn stack_pop_u16(&mut self) -> u16 {
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

    pub(super) fn handle_interrupt(&mut self, interrupt: Interrupt) {
        // TODO: remove this sanity check
        if interrupt.interrupt_type == InterruptType::Nmi
            && self.interrupt_stack.contains(&InterruptType::Nmi)
        {
            self.error = Some(CpuError::InvalidNMI);
            return;
        }

        self.interrupt_stack.push(interrupt.interrupt_type);

        self.stack_push_u16(self.program_counter);

        let mut status_flags = Flags::from_bits_truncate(self.status.bits());
        status_flags.set(Flags::BREAK, interrupt.b_flag_mask & 0b0001_0000 != 0);
        status_flags.set(Flags::BREAK2, interrupt.b_flag_mask & 0b0010_0000 != 0);
        self.stack_push(status_flags.bits());

        self.status.set(Flags::INTERRUPT_DISABLE, true); // Disable interrupts while handling one

        self.extra_cycles += interrupt.cpu_cycles;
        let jmp_address = self.bus_read_u16(interrupt.vector_addr);
        self.set_program_counter(jmp_address);
    }
}

