use std::cmp::PartialEq;
use crate::nes::cpu::interrupts::{Interrupt, InterruptType};
use super::{AddressingMode, Flags, interrupts, opcodes, CPU, CPU_STACK_BASE, CpuError, AddrResult, ExecPhase, AccessType};

impl CPU {

    // Software-defined interrupt
    pub(super) fn brk(&mut self) -> bool {
        let total_cycles = self.current_op.opcode.unwrap().cycles - 1;
        self.current_op.micro_cycle += 1;
        if self.current_op.micro_cycle < total_cycles {
            return false;
        }

        let _ = self.bus_read(self.program_counter); // dummy read

        // BRK - Software-defined Interrupt
        self.program_counter = self.program_counter.wrapping_add(1); // BRK has an implied operand, so increment PC before pushing
        self.handle_interrupt(interrupts::BRK);
        return true
    }

    // General NOP
    pub(super) fn nop(&mut self) -> bool {
        // Don't count initial opcode load
        let cycles_remaining = self.current_op.opcode.unwrap().cycles - 1;
        self.current_op.micro_cycle += 1;
        self.current_op.micro_cycle == cycles_remaining
    }

    pub(super) fn fat_nop(&mut self) -> bool {
        // These are unofficial NOPs that consume extra bytes
        self.exec_read_cycle(|cpu| {})
    }

    //
    // Transfers
    //////////////
    pub(super) fn tax(&mut self) -> bool {
        self.exec_modify_register(|cpu| {
            cpu.set_register_x(cpu.register_a);
        })
    }

    pub(super) fn tay(&mut self) -> bool {
        self.exec_modify_register(|cpu| {
            cpu.set_register_y(cpu.register_a);
        })
    }

    pub(super) fn tsx(&mut self) -> bool {
        self.exec_modify_register(|cpu| {
            cpu.set_register_x(cpu.stack_pointer);
        })
    }

    pub(super) fn txa(&mut self) -> bool {
        self.exec_modify_register(|cpu| {
            cpu.set_register_a(cpu.register_x);
        })
    }

    pub(super) fn txs(&mut self) -> bool {
        self.exec_modify_register(|cpu| {
            cpu.stack_pointer = cpu.register_x;
        })
    }

    pub(super) fn tya(&mut self) -> bool {
        self.exec_modify_register(|cpu| {
            cpu.set_register_a(cpu.register_y);
        })
    }


    //
    // Flags
    //////////
    pub(super) fn sed(&mut self) -> bool {
        self.exec_modify_register(|cpu| {
            cpu.status.insert(Flags::DECIMAL_MODE);
        })
    }

    pub(super) fn sei(&mut self) -> bool {
        self.exec_modify_register(|cpu| {
            cpu.status.insert(Flags::INTERRUPT_DISABLE);
        })
    }
    pub(super) fn sec(&mut self) -> bool {
        self.exec_modify_register(|cpu| {
            cpu.status.insert(Flags::CARRY);
        })
    }

    pub(super) fn cld(&mut self) -> bool {
        self.exec_modify_register(|cpu| {
            cpu.status.remove(Flags::DECIMAL_MODE);
        })
    }

    pub(super) fn cli(&mut self) -> bool {
        self.exec_modify_register(|cpu| {
            cpu.status.remove(Flags::INTERRUPT_DISABLE);
        })
    }
    pub(super) fn clc(&mut self) -> bool {
        self.exec_modify_register(|cpu| {
            cpu.status.remove(Flags::CARRY);
        })
    }

    pub(super) fn clv(&mut self) -> bool {
        self.exec_modify_register(|cpu| {
            cpu.status.remove(Flags::OVERFLOW);
        })
    }

    //
    // Loads
    //////////
    pub(super) fn lda(&mut self) -> bool {
        self.exec_read_cycle(|cpu| {
            cpu.set_register_a(cpu.current_op.tmp_data);
        })
    }

    pub(super) fn ldx(&mut self) -> bool {
        self.exec_read_cycle(|cpu| {
            cpu.set_register_x(cpu.current_op.tmp_data);
        })
    }

    pub(super) fn ldy(&mut self) -> bool {
        self.exec_read_cycle(|cpu| {
            cpu.set_register_y(cpu.current_op.tmp_data);
        })
    }

    //
    // Stores
    //////////
    pub(super) fn sta(&mut self) -> bool {
        self.exec_write_cycle(|cpu| {
            cpu.current_op.tmp_data = cpu.register_a;
        })
    }

    pub(super) fn stx(&mut self) -> bool {
        self.exec_write_cycle(|cpu| {
            cpu.current_op.tmp_data = cpu.register_x;
        })
    }

    pub(super) fn sty(&mut self) -> bool {
        self.exec_write_cycle(|cpu| {
            cpu.current_op.tmp_data = cpu.register_y;
        })
    }

    //
    // Stack
    //////////
    pub(super) fn pla(&mut self) -> bool {
        self.exec_stack_cycle(|cpu| {
            let value = cpu.stack_pop();
            cpu.set_register_a(value);
        })
    }

    pub(super) fn plp(&mut self) -> bool{
        self.exec_stack_cycle(|cpu| {
            // Pop stack into processor_status
            cpu.status = Flags::from_bits_truncate(cpu.stack_pop());
            cpu.status.remove(Flags::BREAK); // This flag is disabled when fetching
            cpu.status.insert(Flags::BREAK2); // This flag is supposed to always be 1 on CPU
        })
    }

    pub(super) fn pha(&mut self) -> bool {
        self.exec_stack_cycle(|cpu| {
            cpu.stack_push(cpu.register_a);
        })
    }

    pub(super) fn php(&mut self) -> bool {
        self.exec_stack_cycle(|cpu| {
            // Push processor_status onto the stack
            // https://www.nesdev.org/wiki/Status_flags
            // says that B flag is pushed as 1, but not affected on the CPU
            let mut status_copy = Flags::from_bits_truncate(cpu.status.bits());
            status_copy.insert(Flags::BREAK);
            cpu.stack_push(status_copy.bits())
        })
    }

    //
    // Shifts
    ///////////
    pub(super) fn asl_reg(&mut self) -> bool {
        // Arithmetic Shift Left into carry
        self.exec_modify_register(|cpu| {
            let carry = cpu.register_a & 0x80 != 0;
            cpu.status.set(Flags::CARRY, carry);
            cpu.set_register_a(cpu.register_a << 1);
        })
    }
    pub(super) fn asl_mem(&mut self) -> bool {
        // Arithmetic Shift Left into carry
        self.exec_read_modify_write_cycle(|cpu| {
            let value = cpu.current_op.tmp_data;
            let carry = value & 0x80 != 0;
            cpu.status.set(Flags::CARRY, carry);
            cpu.current_op.tmp_data = value << 1;
            cpu.update_zero_and_negative_flags(cpu.current_op.tmp_data);
        })
    }

    pub(super) fn lsr_reg(&mut self) -> bool {
        // Logical Shift Right into carry
        self.exec_modify_register(|cpu| {
            let carry = cpu.register_a & 1 != 0;
            cpu.status.set(Flags::CARRY, carry);
            cpu.set_register_a(cpu.register_a >> 1);
        })
    }

    pub(super) fn lsr_mem(&mut self) -> bool {
        self.exec_read_modify_write_cycle(|cpu| {
            let value = cpu.current_op.tmp_data;
            let carry = value & 1 != 0;
            cpu.status.set(Flags::CARRY, carry);
            cpu.current_op.tmp_data = value >> 1;
            cpu.update_zero_and_negative_flags(cpu.current_op.tmp_data);
        })
    }

    //
    // Rotates
    ///////////////
    pub(super) fn rol_reg(&mut self) -> bool {
        // Rotate Left through carry flag
        self.exec_modify_register(|cpu| {
            let carry = cpu.status.contains(Flags::CARRY);
            let (value, new_carry) = Self::rotate_value_left(cpu.register_a, carry);
            cpu.set_register_a(value);
            cpu.status.set(Flags::CARRY, new_carry);
        })
    }

    pub(super) fn rol_mem(&mut self) -> bool {
        // Rotate Left through carry flag
        self.exec_read_modify_write_cycle(|cpu| {
            let value = cpu.current_op.tmp_data;
            let carry = cpu.status.contains(Flags::CARRY);
            let (result, new_carry) = Self::rotate_value_left(value, carry);
            cpu.current_op.tmp_data = result;
            cpu.update_zero_and_negative_flags(result);
            cpu.status.set(Flags::CARRY, new_carry);

        })
    }

    pub(super) fn ror_reg(&mut self) -> bool {
        // Rotate Right through carry flag
        self.exec_modify_register(|cpu| {
            let carry = cpu.status.contains(Flags::CARRY);
            let (value, new_carry) = Self::rotate_value_right(cpu.register_a, carry);
            cpu.set_register_a(value);
            cpu.status.set(Flags::CARRY, new_carry);
        })
    }

    pub(super) fn ror_mem(&mut self) -> bool {
        // Rotate Right through carry flag
        self.exec_read_modify_write_cycle(|cpu| {
            let value = cpu.current_op.tmp_data;
            let carry = cpu.status.contains(Flags::CARRY);
            let (result, new_carry) = Self::rotate_value_right(value, carry);
            cpu.current_op.tmp_data = result;
            cpu.update_zero_and_negative_flags(result);
            cpu.status.set(Flags::CARRY, new_carry);

        })
    }


    //
    // Increments
    ///////////////
    pub(super) fn inc(&mut self) -> bool {
        // Increment value at Memory
        self.exec_read_modify_write_cycle(|cpu| {
            cpu.current_op.tmp_data = cpu.current_op.tmp_data.wrapping_add(1);
            cpu.update_zero_and_negative_flags(cpu.current_op.tmp_data);
        })
    }
    pub(super) fn inx(&mut self) -> bool {
        self.exec_modify_register(|cpu| {
            cpu.register_x = cpu.register_x.wrapping_add(1);
            cpu.update_zero_and_negative_flags(cpu.register_x);
        })
    }

    pub(super) fn iny(&mut self) -> bool {
        self.exec_modify_register(|cpu| {
            cpu.register_y = cpu.register_y.wrapping_add(1);
            cpu.update_zero_and_negative_flags(cpu.register_y);
        })
    }

    //
    // Decrements
    ///////////////
    pub(super) fn dec(&mut self) -> bool {
        // Decrement value at Memory
        self.exec_read_modify_write_cycle(|cpu| {
            cpu.current_op.tmp_data = cpu.current_op.tmp_data.wrapping_sub(1);
            cpu.update_zero_and_negative_flags(cpu.current_op.tmp_data);
        })
    }

    pub(super) fn dex(&mut self) -> bool {
        self.exec_modify_register(|cpu| {
            cpu.register_x = cpu.register_x.wrapping_sub(1);
            cpu.update_zero_and_negative_flags(cpu.register_x);
        })
    }

    pub(super) fn dey(&mut self) -> bool {
        self.exec_modify_register(|cpu| {
            cpu.register_y = cpu.register_y.wrapping_sub(1);
            cpu.update_zero_and_negative_flags(cpu.register_y);
        })
    }

    //
    // Comparisons
    /////////////////
    pub(super) fn compare(&mut self, compare_val: u8) {
        let value = self.current_op.tmp_data;
        self.status.set(Flags::CARRY, compare_val >= value);
        self.update_zero_and_negative_flags(compare_val.wrapping_sub(value));
    }
    pub(super) fn cmp(&mut self) -> bool {
        self.exec_read_cycle(|cpu| {
            cpu.compare(cpu.register_a);
        })
    }
    pub(super) fn cpx(&mut self) -> bool {
        self.exec_read_cycle(|cpu| {
            cpu.compare(cpu.register_x);
        })
    }

    pub(super) fn cpy(&mut self) -> bool {
        self.exec_read_cycle(|cpu| {
            cpu.compare(cpu.register_y);
        })
    }

    // pub(super) fn cmp(&mut self, opcode: &opcodes::Opcode) {
    //     // Compare A register
    //     self.compare(opcode, self.register_a);
    // }

//     pub(super) fn cpx(&mut self, opcode: &opcodes::Opcode) {
//         // Compare X Register
//         self.compare(opcode, self.register_x);
//     }
//
//     pub(super) fn cpy(&mut self, opcode: &opcodes::Opcode) {
//         // Compare Y Register
//         self.compare(opcode, self.register_y);
//     }
//


    pub(super) fn adc(&mut self) -> bool {
        self.exec_read_cycle(|cpu| {
            // Add with Carry
            let value = cpu.current_op.tmp_data;
            cpu.add_to_register_a(value);
        })
    }

    pub(super) fn sbc(&mut self) -> bool {
        self.exec_read_cycle(|cpu| {
            let value = cpu.current_op.tmp_data;
            cpu.sub_from_register_a(value);
        })
    }

    pub(super) fn and(&mut self) -> bool {
        self.exec_read_cycle(|cpu| {
            let value = cpu.current_op.tmp_data;
            cpu.set_register_a(cpu.register_a & value);
        })
    }

    pub(super) fn eor(&mut self) -> bool {
        self.exec_read_cycle(|cpu| {
            let value = cpu.current_op.tmp_data;
            cpu.set_register_a(cpu.register_a ^ value);
        })
    }

    pub(super) fn ora(&mut self) -> bool {
        self.exec_read_cycle(|cpu| {
            let value = cpu.current_op.tmp_data;
            cpu.set_register_a(cpu.register_a | value);
        })
    }


    //
    // Jumps
    ///////////////
    pub(super) fn jmp(&mut self) -> bool {
        self.exec_jmp_cycle(|cpu| {
            cpu.set_program_counter(cpu.current_op.tmp_addr);
        })
    }

    pub(super) fn bne(&mut self) -> bool {
        // Branch if ZERO is clear
        self.exec_branch_cycle(|cpu| !cpu.status.contains(Flags::ZERO))
    }
    pub(super) fn bvs(&mut self) -> bool {
        // Branch if OVERFLOW is set
        self.exec_branch_cycle(|cpu| cpu.status.contains(Flags::OVERFLOW))
    }

    pub(super) fn bvc(&mut self) -> bool {
        // Branch if OVERFLOW is clear
        self.exec_branch_cycle(|cpu| !cpu.status.contains(Flags::OVERFLOW))
    }

    pub(super) fn bmi(&mut self) -> bool {
        // Branch if NEGATIVE is set
        self.exec_branch_cycle(|cpu| cpu.status.contains(Flags::NEGATIVE))
    }

    pub(super) fn beq(&mut self) -> bool {
        // Branch if ZERO is set
        self.exec_branch_cycle(|cpu| cpu.status.contains(Flags::ZERO))
    }

    pub(super) fn bcs(&mut self) -> bool {
        // Branch if CARRY is set
        self.exec_branch_cycle(|cpu| cpu.status.contains(Flags::CARRY))
    }

    pub(super) fn bcc(&mut self) -> bool {
        // Branch if CARRY is clear
        self.exec_branch_cycle(|cpu| !cpu.status.contains(Flags::CARRY))
    }

    pub(super) fn bpl(&mut self) -> bool {
        // Branch if NEGATIVE is clear
        self.exec_branch_cycle(|cpu| !cpu.status.contains(Flags::NEGATIVE))
    }

    //
    // Bit test
    ///////////////
    pub(super) fn bit(&mut self) -> bool {
        self.exec_read_cycle(|cpu| {
            // These flags are set based on bits from the original fetched data
            let value = cpu.current_op.tmp_data;
            cpu.status.set(Flags::NEGATIVE, value & (1 << 7) != 0);
            cpu.status.set(Flags::OVERFLOW, value & (1 << 6) != 0);

            // Update Z flag
            let result = value & cpu.register_a;
            cpu.status.set(Flags::ZERO, result == 0);
        })
    }



//     pub(super) fn jsr(&mut self, opcode: &opcodes::Opcode) {
//         // Jump to Subroutine
//         let (jump_address, _) = self.get_parameter_address(&opcode.mode);
//         let return_address = self.program_counter.wrapping_add(1);
//         self.stack_push_u16(return_address);
//         self.set_program_counter(jump_address);
//     }
//
//     pub(super) fn rts(&mut self) {
//         // Return from Subroutine
//         let return_address_minus_one = self.stack_pop_u16();
//         let address = return_address_minus_one.wrapping_add(1);
//
//         let _ = self.bus_read(self.program_counter); // dummy read
//         self.set_program_counter(address);
//     }
//
//     pub(super) fn rti(&mut self) {
//         // Return from Interrupt
//         // NOTE: Note that unlike RTS, the return address on the stack is the actual address rather than the address-1
//         let return_status = self.stack_pop(); // Restore status flags first
//         let return_address = self.stack_pop_u16(); // Restore PC
//
//         let _ = self.bus_read(self.program_counter); // dummy read
//         self.set_program_counter(return_address);
//
//         let mut restored_flags = Flags::from_bits_truncate(return_status);
//         restored_flags.set(Flags::BREAK, false); // BRK flag is always cleared after RTI
//         restored_flags.set(Flags::BREAK2, true); // BRK2 flag is always cleared after RTI
//         self.status = restored_flags;
//
//         // Pop the most recent interrupt type
//         self.interrupt_stack.pop();
//     }


//
//     pub(super) fn slo(&mut self, opcode: &opcodes::Opcode) {
//         let shifted_result = self.asl(opcode);
//         let ora_result = self.register_a | shifted_result;
//         self.set_register_a(ora_result);
//         self.update_zero_and_negative_flags(ora_result);
//     }
//
//     pub(super) fn nop_page_cross(&mut self, opcode: &opcodes::Opcode) {
//         let (_address, boundary_cross) = self.get_parameter_address(&opcode.mode);
//         self.extra_cycles += boundary_cross as u8;
//     }
// }
//
// /////////////////////////
// // Illegal Opcodes
// /////////////////////////
// impl CPU {
//     pub(super) fn sax(&mut self, opcode: &opcodes::Opcode) {
//         // SAX => A AND X -> M
//         /* A and X are put on the bus at the same time (resulting effectively
//           in an AND operation) and stored in M
//         */
//         let (address, _) = self.get_parameter_address(&opcode.mode);
//         let result = self.register_a & self.register_x;
//         self.bus_write(address, result);
//     }
//
//     pub(super) fn sbx(&mut self, opcode: &opcodes::Opcode) {
//         let (address, _) = self.get_parameter_address(&opcode.mode);
//         let value = self.bus_read(address);
//
//         let and_result = self.register_a & self.register_x;
//         let result = and_result.wrapping_sub(value);
//
//         self.register_x = result;
//         self.status.set(Flags::CARRY, and_result >= value);
//         self.update_zero_and_negative_flags(result);
//     }
//
//     pub(super) fn anc(&mut self, opcode: &opcodes::Opcode) {
//         let (address, _) = self.get_parameter_address(&opcode.mode);
//         let value = self.bus_read(address);
//         self.set_register_a(self.register_a & value);
//         self.status
//             .set(Flags::CARRY, self.register_a & 0b1000_0000 != 0);
//     }
//
//     pub(super) fn arr(&mut self, opcode: &opcodes::Opcode) {
//         // ARR => AND + ROR with special flag behavior
//         let (address, _) = self.get_parameter_address(&opcode.mode);
//         let value = self.bus_read(address);
//         self.register_a &= value;
//
//         // Perform ROR (Rotate Right) with Carry
//         let carry = self.status.contains(Flags::CARRY) as u8;
//         let result = (self.register_a >> 1) | (carry << 7);
//         self.register_a = result;
//         self.update_zero_and_negative_flags(result);
//
//         // Set Carry flag based on bit 6
//         self.status.set(Flags::CARRY, result & 0b0100_0000 != 0);
//
//         // Set Overflow flag based on bits 6 and 5
//         let bit6 = result & 0b0100_0000 != 0;
//         let bit5 = result & 0b0010_0000 != 0;
//         self.status.set(Flags::OVERFLOW, bit6 ^ bit5);
//     }
//

//
//     pub(super) fn sre(&mut self, opcode: &opcodes::Opcode) {
//         // SRE => LSR oper + EOR oper
//         let result = self.lsr(opcode); // LSR
//         self.set_register_a(self.register_a ^ result); // A ^ M -> A
//         self.extra_cycles = 0;
//     }
//
//     pub(super) fn rla(&mut self, opcode: &opcodes::Opcode) {
//         // RLA => ROL oper + AND oper
//         let result = self.rol(opcode); // ROL
//         self.set_register_a(self.register_a & result); // M & A -> A
//         self.extra_cycles = 0;
//     }
//
//     pub(super) fn dcp(&mut self, opcode: &opcodes::Opcode) {
//         // DCP => DEC oper + CMP oper
//         let dec_value = self.dec(opcode);
//
//         // Compare register_a with decremented value
//         let result = self.register_a.wrapping_sub(dec_value);
//         self.status.set(Flags::CARRY, self.register_a >= dec_value);
//         self.update_zero_and_negative_flags(result);
//         self.extra_cycles = 0;
//     }
//
//     pub(super) fn isc(&mut self, opcode: &opcodes::Opcode) {
//         // ISC (ISB / INS) => INC oper + SBC oper
//         let inc_result = self.inc(opcode);
//         self.sub_from_register_a(inc_result);
//     }
//
//     pub(super) fn las(&mut self, opcode: &opcodes::Opcode) {
//         // LAS (LAR) => AND with SP, store in A, X, SP
//         let (address, boundary_crossed) = self.get_parameter_address(&opcode.mode);
//         let value = self.bus_read(address);
//         self.set_register_a(value);
//
//         // Perform AND operation with the stack pointer
//         let result = value & self.stack_pointer;
//         self.register_a = result;
//         self.register_x = result;
//         self.stack_pointer = result;
//
//         self.update_zero_and_negative_flags(result);
//         self.extra_cycles += boundary_crossed as u8;
//     }
//
// }
//
// ///////////////////////////////////////////////////////////////////////////////
// ////// Utility functions
// ///////////////////////////////////////////////////////////////////////////////
// impl CPU {
//     fn get_parameter_address(&mut self, mode: &AddressingMode) -> (u16, bool) {
//         match mode {
//             AddressingMode::Absolute => (self.bus_read_u16(self.program_counter), false),
//             AddressingMode::Immediate => (self.program_counter, false),
//             AddressingMode::ZeroPage => (self.bus_read(self.program_counter) as u16, false),
//             AddressingMode::ZeroPageX => {
//                 let base = self.bus_read(self.program_counter);
//                 let addr = base.wrapping_add(self.register_x) as u16;
//                 (addr, false)
//             }
//             AddressingMode::ZeroPageY => {
//                 let base = self.bus_read(self.program_counter);
//                 let addr = base.wrapping_add(self.register_y) as u16;
//                 (addr, false)
//             }
//             AddressingMode::AbsoluteX => {
//                 let base = self.bus_read_u16(self.program_counter);
//                 let addr = base.wrapping_add(self.register_x as u16);
//
//                 // Only read from base page (not the final address)
//                 let dummy_addr =
//                     (base & 0xFF00) | ((base.wrapping_add(self.register_y as u16)) & 0x00FF);
//                 let _ = self.bus_read(dummy_addr);
//
//                 (addr, Self::is_boundary_crossed(base, addr))
//             }
//             AddressingMode::AbsoluteY => {
//                 let base = self.bus_read_u16(self.program_counter);
//                 let addr = base.wrapping_add(self.register_y as u16);
//
//                 // Only read from base page (not the final address)
//                 let dummy_addr =
//                     (base & 0xFF00) | ((base.wrapping_add(self.register_y as u16)) & 0x00FF);
//                 let _ = self.bus_read(dummy_addr);
//
//                 (addr, Self::is_boundary_crossed(base, addr))
//             }
//             AddressingMode::IndirectX => {
//                 let base = self.bus_read(self.program_counter);
//                 let addr = base.wrapping_add(self.register_x); // Zero-page wrapping
//                 let lo = self.bus_read(addr as u16) as u16;
//                 let hi = self.bus_read(addr.wrapping_add(1) as u16) as u16; // Zero-page wrap +1 as well
//                 (hi << 8 | lo, false)
//             }
//             AddressingMode::IndirectY => {
//                 let base = self.bus_read(self.program_counter) as u16;
//                 let lo = self.bus_read(base) as u16;
//                 let hi = self.bus_read((base as u8).wrapping_add(1) as u16) as u16;
//                 let dynamic_base = hi << 8 | lo;
//                 let addr = dynamic_base.wrapping_add(self.register_y as u16);
//                 (addr, Self::is_boundary_crossed(dynamic_base, addr))
//             }
//             AddressingMode::Indirect => {
//                 // Note: JMP is the only opcode to use this AddressingMode
//                 /* NOTE:
//                   An original 6502 has does not correctly fetch the target address if the indirect vector falls
//                   on a page boundary (e.g. $xxFF where xx is any value from $00 to $FF). In this case fetches
//                   the LSB from $xxFF as expected but takes the MSB from $xx00.
//                 */
//                 let indirect_vec = self.bus_read_u16(self.program_counter);
//                 let address = if indirect_vec & 0x00FF == 0x00FF {
//                     let lo = self.bus_read(indirect_vec) as u16;
//                     let hi = self.bus_read(indirect_vec & 0xFF00) as u16;
//                     (hi << 8) | lo
//                 } else {
//                     self.bus_read_u16(indirect_vec)
//                 };
//                 (address, false)
//             }
//             AddressingMode::Relative => {
//                 // Note: Branch opcodes exclusively use this address mode
//                 let offset = self.bus_read(self.program_counter) as i8; // sign-extend u8 to i8
//                 let base_pc = self.program_counter.wrapping_add(1); // the relative address is based on a PC /after/ the current opcode
//                 let target_address = base_pc.wrapping_add_signed(offset as i16);
//                 let boundary_crossed = Self::is_boundary_crossed(base_pc, target_address);
//                 (target_address, boundary_crossed)
//             }
//             _ => unimplemented!(),
//         }
//     }
//     fn is_boundary_crossed(addr1: u16, addr2: u16) -> bool {
//         addr1 & 0xFF00 != addr2 & 0xFF00
//     }
//
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

    fn branch(&mut self, condition: bool) {
        if condition {
            let addr = self.current_op.tmp_addr;
            self.set_program_counter(addr);
        }
    }

   // fn branch(&mut self, opcode: &opcodes::Opcode, condition: bool) {
   //      let (address, boundary_crossed) = self.get_parameter_address(&opcode.mode);
   //      let cycles = boundary_crossed as u8;
   //      if condition {
   //          self.set_program_counter(address);
   //          self.extra_cycles = self.extra_cycles + cycles + 1;
   //      }
   //  }

    pub(super) fn handle_interrupt(&mut self, interrupt: Interrupt) {
        // TODO: remove this sanity check
        // if interrupt.interrupt_type == InterruptType::Nmi
        //     && self.interrupt_stack.contains(&InterruptType::Nmi)
        // {
        //     self.error = Some(CpuError::InvalidNMI);
        //     return;
        // }

        self.interrupt_stack.push(interrupt.interrupt_type);

        self.stack_push_u16(self.program_counter);

        let mut status_flags = Flags::from_bits_truncate(self.status.bits());
        status_flags.set(Flags::BREAK, interrupt.b_flag_mask & 0b0001_0000 != 0);
        status_flags.set(Flags::BREAK2, interrupt.b_flag_mask & 0b0010_0000 != 0);
        self.stack_push(status_flags.bits());

        self.status.set(Flags::INTERRUPT_DISABLE, true); // Disable interrupts while handling one

        // self.extra_cycles += interrupt.cpu_cycles;
        let jmp_address = self.bus_read_u16(interrupt.vector_addr);
        self.set_program_counter(jmp_address);
    }

}

impl CPU {

    fn needs_dummy_cycle(&mut self) -> bool {
        match self.current_op.access_type {
            AccessType::Read => self.current_op.page_crossed,
            _ => true,
        }
    }
    fn tick_addressing_mode(&mut self) -> AddrResult {
        let mode = self.current_op.opcode.unwrap().mode;
        match mode {
            AddressingMode::Immediate => {
                match self.current_op.micro_cycle {
                    0 => {
                        self.current_op.tmp_data = self.consume_program_counter();
                        // self.current_op.exec_phase = ExecPhase::Read;
                        self.current_op.micro_cycle += 1;
                        return AddrResult::ReadyImmediate(self.current_op.tmp_data);
                    }
                    _ => unreachable!(),
                }
            }
            AddressingMode::ZeroPage => {
                return match self.current_op.micro_cycle {
                    0 => {
                        let zero_page = self.consume_program_counter();
                        self.current_op.tmp_addr = zero_page as u16;
                        self.current_op.micro_cycle += 1;
                        AddrResult::Ready(self.current_op.tmp_addr)
                    }
                    _ => {
                        AddrResult::Ready(self.current_op.tmp_addr)
                    }
                }
            }
            AddressingMode::ZeroPageX | AddressingMode::ZeroPageY => {
                let index = if mode == AddressingMode::ZeroPageX {
                    self.register_x
                } else {
                    self.register_y
                };
                match self.current_op.micro_cycle {
                    0 => {
                        let zero_page = self.consume_program_counter();
                        self.current_op.tmp_addr = zero_page as u16;
                    }
                    1 => {
                        let _ = self.bus_read(self.current_op.tmp_addr); // dummy read
                        self.current_op.tmp_addr = self.current_op.tmp_addr.wrapping_add(index as u16) & 0x00FF;

                        self.current_op.micro_cycle += 1;
                        return AddrResult::Ready(self.current_op.tmp_addr);
                    }
                    2 => {
                        return AddrResult::Ready(self.current_op.tmp_addr);
                    }
                    _ => unreachable!(),
                }
            }
            AddressingMode::Absolute => {
                match self.current_op.micro_cycle {
                    0 => {
                        let lo = self.consume_program_counter();
                        self.current_op.tmp_addr = lo as u16;
                    }
                    1 => {
                        let hi = self.consume_program_counter();
                        self.current_op.tmp_addr |= (hi as u16) << 8;
                        self.current_op.micro_cycle += 1;
                        return AddrResult::Ready(self.current_op.tmp_addr);
                    }
                    _ => {
                        return AddrResult::Ready(self.current_op.tmp_addr);
                    }
                }
            }
            AddressingMode::AbsoluteX | AddressingMode::AbsoluteY => {
                let index = if mode == AddressingMode::AbsoluteX {
                    self.register_x
                } else {
                    self.register_y
                };

                match self.current_op.micro_cycle {
                    0 => {
                        let lo = self.consume_program_counter();
                        self.current_op.tmp_addr = lo as u16;
                    }
                    1 => {
                        let hi = self.consume_program_counter();

                        let base = self.current_op.tmp_addr | ((hi as u16) << 8);
                        let addr = base.wrapping_add(index as u16);

                        self.current_op.page_crossed = (base & 0xFF00) != (addr & 0xFF00);
                        self.current_op.tmp_addr = addr;

                        if !self.needs_dummy_cycle() {
                            self.current_op.micro_cycle += 1;
                            return AddrResult::Ready(self.current_op.tmp_addr);
                        }
                    }
                    2 => {
                        if self.needs_dummy_cycle() {
                            // dummy read
                            let dummy = (self.current_op.tmp_addr & 0xFF00)
                                | ((self.current_op.tmp_addr.wrapping_sub(index as u16)) & 0x00FF);
                            let _ = self.bus_read(dummy);
                        }
                        self.current_op.micro_cycle += 1;
                        return AddrResult::Ready(self.current_op.tmp_addr);
                    }
                    3 => {
                        return AddrResult::Ready(self.current_op.tmp_addr);
                    }
                    _ => unreachable!()
                }
            }
            AddressingMode::IndirectX => {
                match self.current_op.micro_cycle {
                    0 => {
                        let zero_page = self.consume_program_counter();
                        self.current_op.tmp_addr = zero_page as u16;
                    }
                    1 => {
                        // dummy read from zero page
                        let _ = self.bus_read(self.current_op.tmp_addr);
                    }
                    2 => {
                        let addr = self.current_op.tmp_addr.wrapping_add(self.register_x as u16) & 0x00FF;
                        let lo = self.bus_read(addr);
                        self.current_op.tmp_data = lo;
                        self.current_op.tmp_addr = addr;
                    }
                    3 => {
                        let hi = self.bus_read((self.current_op.tmp_addr + 1) & 0x00FF);
                        let final_addr = ((hi as u16) << 8) | self.current_op.tmp_data as u16;
                        self.current_op.tmp_addr = final_addr;
                        self.current_op.micro_cycle += 1;
                        return AddrResult::Ready(self.current_op.tmp_addr);
                    }
                    4 => {
                        return AddrResult::Ready(self.current_op.tmp_addr);
                    }
                    _ => unreachable!()
                }
            }
            AddressingMode::IndirectY => {
                // Note: JMP is the only opcode to use this AddressingMode
                match self.current_op.micro_cycle {
                    0 => {
                        let zero_page = self.consume_program_counter();
                        self.current_op.tmp_addr = zero_page as u16;
                    }
                    1 => {
                        let lo = self.bus_read(self.current_op.tmp_addr & 0x00FF);
                        self.current_op.tmp_data = lo;
                    }
                    2 => {
                        let hi = self.bus_read((self.current_op.tmp_addr + 1) & 0x00FF);
                        let base = (hi as u16) << 8 | self.current_op.tmp_data as u16;
                        let addr = base.wrapping_add(self.register_y as u16);

                        self.current_op.page_crossed = (base & 0xFF00) != (addr & 0xFF00);
                        self.current_op.tmp_addr = addr;

                        if !self.needs_dummy_cycle() {
                            self.current_op.micro_cycle += 1;
                            return AddrResult::Ready(self.current_op.tmp_addr);
                        }
                    }
                    3 => {
                        if self.needs_dummy_cycle() {
                            let dummy = (self.current_op.tmp_addr & 0xFF00)
                                | ((self.current_op.tmp_addr.wrapping_sub(self.register_y as u16)) & 0x00FF);
                            let _ = self.bus_read(dummy);
                        }
                        self.current_op.micro_cycle += 1;
                        return AddrResult::Ready(self.current_op.tmp_addr);
                    }
                    4 => {
                        return AddrResult::Ready(self.current_op.tmp_addr);
                    }
                    _ => {}
                }
            }
            AddressingMode::Indirect => {
                match self.current_op.micro_cycle {
                    0 => {
                        let lo = self.consume_program_counter();
                        self.current_op.tmp_addr = lo as u16;
                    }
                    1 => {
                        let hi = self.consume_program_counter();
                        self.current_op.tmp_addr |= (hi as u16) << 8;
                    }
                    2 => {
                        let lo = self.bus_read(self.current_op.tmp_addr);
                        self.current_op.tmp_data = lo;
                    }
                    3 => {
                        let hi_addr = (self.current_op.tmp_addr & 0xFF00)
                            | ((self.current_op.tmp_addr + 1) & 0x00FF); // hardware bug

                        let hi = self.bus_read(hi_addr);
                        self.current_op.tmp_addr = (hi as u16) << 8 | self.current_op.tmp_data as u16;
                        return AddrResult::Ready(self.current_op.tmp_addr);
                    }
                    _ => {}
                }
            }
            AddressingMode::Relative => {
                // // Note: Branch opcodes exclusively use this address mode
                // let offset = self.bus_read(self.program_counter) as i8; // sign-extend u8 to i8
                // let base_pc = self.program_counter.wrapping_add(1); // the relative address is based on a PC /after/ the current opcode
                // let target_address = base_pc.wrapping_add_signed(offset as i16);
                // let boundary_crossed = Self::is_boundary_crossed(base_pc, target_address);
                // (target_address, boundary_crossed)

                match self.current_op.micro_cycle {
                    0 => {
                        self.current_op.tmp_data = self.consume_program_counter();
                        self.current_op.micro_cycle += 1;
                        return AddrResult::Ready(self.current_op.tmp_addr);
                    }
                    1 => {
                        return AddrResult::Ready(self.current_op.tmp_addr);
                    }
                    _ => {}
                }
            }
            _ => unreachable!("unsupported addressing mode"),
        }
        self.current_op.micro_cycle += 1;
        AddrResult::InProgress
    }
}

impl CPU {
    fn exec_read_cycle<F>(&mut self, op: F) -> bool
    where
        F: Fn(&mut CPU),
    {
        match self.tick_addressing_mode() {
            AddrResult::InProgress => false,
            AddrResult::Ready(addr) => {
                match self.current_op.exec_phase {
                    ExecPhase::Idle => {
                        self.current_op.exec_phase = ExecPhase::Read;
                        false
                    }
                    ExecPhase::Read => {
                        self.current_op.tmp_data = self.bus_read(addr);
                        op(self);
                        self.current_op.exec_phase = ExecPhase::Done;
                        true
                    }
                    _ => unreachable!(),
                }
            }
            AddrResult::ReadyImmediate(val) => {
                op(self);
                true
            }
        }
    }

    fn exec_write_cycle<F>(&mut self, op: F) -> bool
    where
        F: Fn(&mut CPU),
    {
        match self.tick_addressing_mode() {
            AddrResult::InProgress => false,
            AddrResult::Ready(addr) => {
                match self.current_op.exec_phase {
                    ExecPhase::Idle => {
                        self.current_op.exec_phase = ExecPhase::Write;
                        false
                    }
                    ExecPhase::Write => {
                        op(self);
                        self.bus_write(addr, self.current_op.tmp_data);
                        self.current_op.exec_phase = ExecPhase::Done;
                        true
                    }
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        }
    }

    fn exec_read_modify_write_cycle<F>(&mut self, op: F) -> bool
    where
        F: Fn(&mut CPU),
    {
        match self.tick_addressing_mode() {
            AddrResult::InProgress => false,
            AddrResult::Ready(addr) => {
                match self.current_op.exec_phase {
                    ExecPhase::Idle => {
                        self.current_op.exec_phase = ExecPhase::Read;
                        false
                    }
                    ExecPhase::Read => {
                        self.current_op.tmp_data = self.bus_read(addr);
                        self.current_op.exec_phase = ExecPhase::Internal;
                        false
                    }
                    ExecPhase::Internal => {
                        self.bus_write(addr, self.current_op.tmp_data); // dummy write
                        op(self);
                        self.current_op.exec_phase = ExecPhase::Write;
                        false
                    }
                    ExecPhase::Write => {
                        self.bus_write(addr, self.current_op.tmp_data);
                        self.current_op.exec_phase = ExecPhase::Done;
                        true
                    }
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        }
    }

    fn exec_stack_cycle<F>(&mut self, op: F) -> bool
    where
        F: Fn(&mut CPU),
    {
        match self.current_op.exec_phase {
            ExecPhase::Idle => {
                self.current_op.exec_phase = ExecPhase::Write;
                false
            }
            ExecPhase::Write => {
                op(self);
                self.current_op.exec_phase = ExecPhase::Done;
                true
            }
            _ => unreachable!(),
        }
    }

    fn exec_modify_register<F>(&mut self, op: F) -> bool
    where
        F: Fn(&mut CPU),
    {
        op(self);
        self.current_op.exec_phase = ExecPhase::Done;
        true
    }

    fn exec_jmp_cycle<F>(&mut self, op: F) -> bool
    where
        F: Fn(&mut CPU),
    {
        match self.tick_addressing_mode() {
            AddrResult::InProgress => false,
            AddrResult::Ready(addr) => {
                op(self);
                true
            }
            AddrResult::ReadyImmediate(val) => {
                op(self);
                true
            }
        }
    }

    fn exec_branch_cycle<F>(&mut self, condition: F) -> bool
    where
        F: Fn(&mut CPU) -> bool,
    {
        match self.tick_addressing_mode() {
            AddrResult::InProgress => false,
            AddrResult::Ready(addr) => {
                match self.current_op.exec_phase {
                    ExecPhase::Idle => {
                        if condition(self) { // branch succeed
                            self.current_op.exec_phase = ExecPhase::Internal;
                            false
                        } else { // branch fail
                            self.current_op.exec_phase = ExecPhase::Done;
                            true
                        }
                    }
                    ExecPhase::Internal => {
                        // calculate new branch location and move to it
                        let offset = self.current_op.tmp_data as i8;
                        let old_pc = self.program_counter;
                        let new_pc = old_pc.wrapping_add(offset as u16);
                        self.set_program_counter(new_pc);

                        let page_crossed = (old_pc & 0xFF00) != (new_pc & 0xFF00);
                        if page_crossed {
                            self.current_op.page_crossed = page_crossed;
                            self.current_op.exec_phase = ExecPhase::Write;
                            false
                        } else {
                            self.current_op.exec_phase = ExecPhase::Done;
                            true
                        }
                    }
                    ExecPhase::Write => {
                        self.current_op.exec_phase = ExecPhase::Done;
                        true
                    }
                    ExecPhase::Done => true,
                    _ => unreachable!()
                }
            }
            _ => unreachable!(),
        }
    }
}