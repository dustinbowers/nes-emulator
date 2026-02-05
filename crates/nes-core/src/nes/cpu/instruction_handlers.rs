use super::{
    AccessType, AddrResult, AddressingMode, CPU, CPU_STACK_BASE, CpuError, ExecPhase, Flags,
    Interrupt,
};
use crate::nes::cpu::interrupts::InterruptType;
use crate::trace_cpu_event;

impl CPU {
    /// Software-defined interrupt
    pub(super) fn brk(&mut self) -> bool {
        match self.current_op.micro_cycle {
            0 => {
                let _ = self.read_program_counter(); // dummy read
                self.current_op.tmp_addr = self.program_counter.wrapping_add(1);
            }
            1 => {
                let hi = self.current_op.tmp_addr >> 8;
                self.stack_push(hi as u8);
            }
            2 => {
                let lo = self.current_op.tmp_addr;
                self.stack_push(lo as u8);
            }
            3 => {
                let mut status_flags = Flags::from_bits_truncate(self.status.bits());
                status_flags.set(Flags::BREAK, true);
                status_flags.set(Flags::BREAK2, true);
                self.stack_push(status_flags.bits());
                self.status.insert(Flags::INTERRUPT_DISABLE); // Disable interrupts while handling one
            }
            4 => {
                let lo = self.bus_read(0xFFFE);
                self.current_op.tmp_addr = lo as u16;
            }
            5 => {
                let hi = self.bus_read(0xFFFF);
                let lo = self.current_op.tmp_addr;
                let vector = ((hi as u16) << 8) | lo;
                self.set_program_counter(vector);
                return true;
            }
            _ => unreachable!(),
        }
        self.current_op.micro_cycle += 1;
        false
    }

    /// General NOP
    pub(super) fn nop(&mut self) -> bool {
        // Don't count initial opcode load
        let cycles_remaining = self.current_op.opcode.unwrap().cycles - 1;
        self.current_op.micro_cycle += 1;

        let done = self.current_op.micro_cycle == cycles_remaining;
        if done {
            self.current_op.exec_phase = ExecPhase::Done;
        }
        done
    }

    /// Unofficial NOPs that consume extra bytes
    pub(super) fn fat_nop(&mut self) -> bool {
        self.exec_read_cycle(|_| {})
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

    /// Set decimal_mode flag
    pub(super) fn sed(&mut self) -> bool {
        self.exec_modify_register(|cpu| {
            cpu.status.insert(Flags::DECIMAL_MODE);
        })
    }

    /// Set interrupt_disable flag
    pub(super) fn sei(&mut self) -> bool {
        self.exec_modify_register(|cpu| {
            cpu.status.insert(Flags::INTERRUPT_DISABLE);
        })
    }

    /// Set carry flag
    pub(super) fn sec(&mut self) -> bool {
        self.exec_modify_register(|cpu| {
            cpu.status.insert(Flags::CARRY);
        })
    }

    /// Clear decimal mode flag
    pub(super) fn cld(&mut self) -> bool {
        self.exec_modify_register(|cpu| {
            cpu.status.remove(Flags::DECIMAL_MODE);
        })
    }

    /// Clear interrupt_disable flag
    pub(super) fn cli(&mut self) -> bool {
        self.exec_modify_register(|cpu| {
            cpu.status.remove(Flags::INTERRUPT_DISABLE);
        })
    }

    /// Clear carry flag
    pub(super) fn clc(&mut self) -> bool {
        self.exec_modify_register(|cpu| {
            cpu.status.remove(Flags::CARRY);
        })
    }

    /// Clear overflow flag
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

    /// Pop stack into accumulator
    pub(super) fn pla(&mut self) -> bool {
        self.exec_stack_pop_cycle(|cpu| {
            let value = cpu.stack_pop();
            cpu.set_register_a(value);
        })
    }

    /// Pop stack into processor_status
    pub(super) fn plp(&mut self) -> bool {
        self.exec_stack_pop_cycle(|cpu| {
            cpu.status = Flags::from_bits_truncate(cpu.stack_pop());
            cpu.status.remove(Flags::BREAK); // This flag is disabled when fetching
            cpu.status.insert(Flags::BREAK2); // This flag is supposed to always be 1 on CPU
        })
    }

    /// Push accumulator onto stack
    pub(super) fn pha(&mut self) -> bool {
        self.exec_stack_push_cycle(|cpu| {
            cpu.stack_push(cpu.register_a);
        })
    }

    /// Push status register onto stack
    pub(super) fn php(&mut self) -> bool {
        self.exec_stack_push_cycle(|cpu| {
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

    /// Arithmetic Shift Left into carry
    pub(super) fn asl_reg(&mut self) -> bool {
        self.exec_modify_register(|cpu| {
            let carry = cpu.register_a & 0x80 != 0;
            cpu.status.set(Flags::CARRY, carry);
            cpu.set_register_a(cpu.register_a << 1);
        })
    }

    /// Arithmetic Shift Left into carry
    pub(super) fn asl_mem(&mut self) -> bool {
        self.exec_read_modify_write_cycle(|cpu| {
            let value = cpu.current_op.tmp_data;
            let carry = value & 0x80 != 0;
            cpu.status.set(Flags::CARRY, carry);
            cpu.current_op.tmp_data = value << 1;
            cpu.update_zero_and_negative_flags(cpu.current_op.tmp_data);
        })
    }

    /// Logical Shift Right into carry (register)
    pub(super) fn lsr_reg(&mut self) -> bool {
        self.exec_modify_register(|cpu| {
            let carry = cpu.register_a & 1 != 0;
            cpu.status.set(Flags::CARRY, carry);
            cpu.set_register_a(cpu.register_a >> 1);
        })
    }

    /// Logical Shift Right into carry (memory)
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

    /// Rotate Left through carry flag
    pub(super) fn rol_reg(&mut self) -> bool {
        self.exec_modify_register(|cpu| {
            let carry = cpu.status.contains(Flags::CARRY);
            let (value, new_carry) = Self::rotate_value_left(cpu.register_a, carry);
            cpu.set_register_a(value);
            cpu.status.set(Flags::CARRY, new_carry);
        })
    }

    /// Rotate Left through carry flag
    pub(super) fn rol_mem(&mut self) -> bool {
        self.exec_read_modify_write_cycle(|cpu| {
            let value = cpu.current_op.tmp_data;
            let carry = cpu.status.contains(Flags::CARRY);
            let (result, new_carry) = Self::rotate_value_left(value, carry);
            cpu.current_op.tmp_data = result;
            cpu.update_zero_and_negative_flags(result);
            cpu.status.set(Flags::CARRY, new_carry);
        })
    }

    /// Rotate Right through carry flag
    pub(super) fn ror_reg(&mut self) -> bool {
        self.exec_modify_register(|cpu| {
            let carry = cpu.status.contains(Flags::CARRY);
            let (value, new_carry) = Self::rotate_value_right(cpu.register_a, carry);
            cpu.set_register_a(value);
            cpu.status.set(Flags::CARRY, new_carry);
        })
    }

    /// Rotate Right through carry flag
    pub(super) fn ror_mem(&mut self) -> bool {
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

    /// Increment value at Memory
    pub(super) fn inc(&mut self) -> bool {
        self.exec_read_modify_write_cycle(|cpu| {
            cpu.current_op.tmp_data = cpu.current_op.tmp_data.wrapping_add(1);
            cpu.update_zero_and_negative_flags(cpu.current_op.tmp_data);
        })
    }

    /// Increment register X
    pub(super) fn inx(&mut self) -> bool {
        self.exec_modify_register(|cpu| {
            cpu.register_x = cpu.register_x.wrapping_add(1);
            cpu.update_zero_and_negative_flags(cpu.register_x);
        })
    }

    /// Increment register Y
    pub(super) fn iny(&mut self) -> bool {
        self.exec_modify_register(|cpu| {
            cpu.register_y = cpu.register_y.wrapping_add(1);
            cpu.update_zero_and_negative_flags(cpu.register_y);
        })
    }

    //
    // Decrements
    ///////////////

    /// Decrement value at Memory
    pub(super) fn dec(&mut self) -> bool {
        self.exec_read_modify_write_cycle(|cpu| {
            cpu.current_op.tmp_data = cpu.current_op.tmp_data.wrapping_sub(1);
            cpu.update_zero_and_negative_flags(cpu.current_op.tmp_data);
        })
    }

    /// Decrement register x
    pub(super) fn dex(&mut self) -> bool {
        self.exec_modify_register(|cpu| {
            cpu.register_x = cpu.register_x.wrapping_sub(1);
            cpu.update_zero_and_negative_flags(cpu.register_x);
        })
    }

    // Decrement register y
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

    //
    // Addition/Subtraction
    ///////////////////////////
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

    //
    // Bitwise ops
    //////////////////
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
    /// Unconditional jump
    pub(super) fn jmp(&mut self) -> bool {
        self.exec_jmp_cycle(|cpu| {
            cpu.set_program_counter(cpu.current_op.tmp_addr);
        })
    }

    /// Jump to subroutine
    pub(super) fn jsr(&mut self) -> bool {
        match self.tick_addressing_mode() {
            AddrResult::InProgress => false,
            AddrResult::Ready(addr) => {
                match self.current_op.exec_phase {
                    ExecPhase::Idle => {
                        self.current_op.exec_phase = ExecPhase::Read;
                        false
                    }
                    ExecPhase::Read => {
                        self.bus_read(addr); // dummy read cycle
                        self.current_op.exec_phase = ExecPhase::Internal;
                        false
                    }
                    ExecPhase::Internal => {
                        let pc = self.program_counter.wrapping_sub(1);
                        let hi = (pc >> 8) as u8;
                        self.stack_push(hi);
                        self.current_op.exec_phase = ExecPhase::Write;
                        false
                    }
                    ExecPhase::Write => {
                        let pc = self.program_counter.wrapping_sub(1);
                        let lo = pc as u8;
                        self.stack_push(lo);
                        self.set_program_counter(addr);
                        self.current_op.exec_phase = ExecPhase::Done;
                        true
                    }
                    _ => unreachable!(),
                }
            }
            _ => true,
        }
    }

    //
    // Returns
    //////////////
    /// Return from subroutine
    pub(super) fn rts(&mut self) -> bool {
        // Return from subroutine
        match self.current_op.micro_cycle {
            0 => {
                let _ = self.read_program_counter(); // dummy read
            }
            1 => {
                let lo = self.stack_pop();
                self.current_op.tmp_addr = lo as u16;
            }
            2 => {
                let hi = self.stack_pop();
                let lo = self.current_op.tmp_addr;
                let addr = (hi as u16) << 8 | lo;
                self.current_op.tmp_addr = addr;
            }
            3 => {
                self.set_program_counter(self.current_op.tmp_addr);
                self.advance_program_counter();
            }
            4 => {
                let _ = self.read_program_counter(); // dummy read
                return true;
            }
            _ => unreachable!(),
        }
        self.current_op.micro_cycle += 1;
        false
    }

    /// Return from interrupt
    pub(super) fn rti(&mut self) -> bool {
        match self.current_op.micro_cycle {
            0 => {
                let _ = self.read_program_counter(); // dummy read
            }
            1 => {
                // Restore status flags
                let status = self.stack_pop();
                let mut flags = Flags::from_bits_truncate(status);
                flags.set(Flags::BREAK, false); // BRK is always cleared after RTI
                flags.set(Flags::BREAK2, true); // BRK2 is always cleared after RI
                self.status = flags;
            }
            2 => {
                let lo = self.stack_pop();
                self.current_op.tmp_addr = lo as u16;
            }
            3 => {
                let hi = self.stack_pop();
                let lo = self.current_op.tmp_addr;
                let return_addr = ((hi as u16) << 8) | lo;

                // Restore PC
                self.set_program_counter(return_addr);
            }
            4 => {
                let _ = self.read_program_counter(); // dummy read
                return true;
            }
            _ => unreachable!(),
        }
        self.current_op.micro_cycle += 1;
        false
    }

    //
    // Branches
    ////////////////
    /// Branch if ZERO is clear
    pub(super) fn bne(&mut self) -> bool {
        self.exec_branch_cycle(|cpu| !cpu.status.contains(Flags::ZERO))
    }

    /// Branch if OVERFLOW is set
    pub(super) fn bvs(&mut self) -> bool {
        self.exec_branch_cycle(|cpu| cpu.status.contains(Flags::OVERFLOW))
    }

    /// Branch if OVERFLOW is clear
    pub(super) fn bvc(&mut self) -> bool {
        self.exec_branch_cycle(|cpu| !cpu.status.contains(Flags::OVERFLOW))
    }

    /// Branch if NEGATIVE is set
    pub(super) fn bmi(&mut self) -> bool {
        self.exec_branch_cycle(|cpu| cpu.status.contains(Flags::NEGATIVE))
    }

    /// Branch if ZERO is set
    pub(super) fn beq(&mut self) -> bool {
        self.exec_branch_cycle(|cpu| cpu.status.contains(Flags::ZERO))
    }

    /// Branch if CARRY is set
    pub(super) fn bcs(&mut self) -> bool {
        self.exec_branch_cycle(|cpu| cpu.status.contains(Flags::CARRY))
    }

    /// Branch if CARRY is clear
    pub(super) fn bcc(&mut self) -> bool {
        self.exec_branch_cycle(|cpu| !cpu.status.contains(Flags::CARRY))
    }

    /// Branch if NEGATIVE is clear
    pub(super) fn bpl(&mut self) -> bool {
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
}

////////////////////////////////
// Unofficial Opcodes
////////////////////////////////
impl CPU {
    /// DCP => DEC oper + CMP oper
    pub(super) fn dcp(&mut self) -> bool {
        self.exec_read_modify_write_cycle(|cpu| {
            // DEC
            let value = cpu.current_op.tmp_data.wrapping_sub(1);
            cpu.current_op.tmp_data = value;
            cpu.update_zero_and_negative_flags(value);

            // Compare register_a with decremented value
            let result = cpu.register_a.wrapping_sub(value);
            cpu.status.set(Flags::CARRY, cpu.register_a >= value);
            cpu.update_zero_and_negative_flags(result);
        })
    }

    /// RLA => ROL oper + AND oper
    pub(super) fn rla(&mut self) -> bool {
        self.exec_read_modify_write_cycle(|cpu| {
            // ROL
            let value = cpu.current_op.tmp_data;
            let carry = cpu.status.contains(Flags::CARRY);
            let (result, new_carry) = Self::rotate_value_left(value, carry);
            cpu.current_op.tmp_data = result;
            cpu.update_zero_and_negative_flags(result);
            cpu.status.set(Flags::CARRY, new_carry);

            // AND
            cpu.set_register_a(cpu.register_a & result);
        })
    }

    /// SLO => ASL oper + ORA oper
    pub(super) fn slo(&mut self) -> bool {
        self.exec_read_modify_write_cycle(|cpu| {
            // ASL
            let value = cpu.current_op.tmp_data;
            let carry = value & 0x80 != 0;
            cpu.status.set(Flags::CARRY, carry);
            cpu.current_op.tmp_data = value << 1;
            cpu.update_zero_and_negative_flags(cpu.current_op.tmp_data);

            // ORA
            let ora_result = cpu.register_a | cpu.current_op.tmp_data;
            cpu.set_register_a(ora_result);
        })
    }

    /// SRE => LSR oper + EOR oper
    pub(super) fn sre(&mut self) -> bool {
        self.exec_read_modify_write_cycle(|cpu| {
            // LSR
            let value = cpu.current_op.tmp_data;
            let carry = value & 1 != 0;
            cpu.status.set(Flags::CARRY, carry);
            cpu.current_op.tmp_data = value >> 1;
            cpu.update_zero_and_negative_flags(cpu.current_op.tmp_data);

            // EOR
            let value = cpu.current_op.tmp_data;
            cpu.set_register_a(cpu.register_a ^ value);
        })
    }

    /// RRA => ROR oper + ADC oper
    pub(super) fn rra(&mut self) -> bool {
        self.exec_read_modify_write_cycle(|cpu| {
            // ROR
            let value = cpu.current_op.tmp_data;
            let carry = cpu.status.contains(Flags::CARRY);
            let (result, new_carry) = Self::rotate_value_right(value, carry);
            cpu.current_op.tmp_data = result;
            cpu.update_zero_and_negative_flags(result);
            cpu.status.set(Flags::CARRY, new_carry);

            // ADC
            let value = cpu.current_op.tmp_data;
            cpu.add_to_register_a(value);
        })
    }

    /// ISC => INC oper + SBC oper
    pub(super) fn isc(&mut self) -> bool {
        self.exec_read_modify_write_cycle(|cpu| {
            // INC
            cpu.current_op.tmp_data = cpu.current_op.tmp_data.wrapping_add(1);
            cpu.update_zero_and_negative_flags(cpu.current_op.tmp_data);

            // SBC
            let value = cpu.current_op.tmp_data;
            cpu.sub_from_register_a(value);
        })
    }

    /// LAX => LDA oper + LDX oper
    pub(super) fn lax(&mut self) -> bool {
        self.exec_read_cycle(|cpu| {
            cpu.set_register_a(cpu.current_op.tmp_data); // LDA
            cpu.set_register_x(cpu.current_op.tmp_data); // LDX
        })
    }

    /// SAX => A AND X -> M
    pub(super) fn sax(&mut self) -> bool {
        self.exec_write_cycle(|cpu| {
            let a = cpu.register_a;
            let x = cpu.register_x;
            cpu.current_op.tmp_data = a & x;
        })
    }

    /// SBX (AXS, SAX) => CMP and DEX at once, sets flags like CMP
    pub(super) fn sbx(&mut self) -> bool {
        self.exec_read_cycle(|cpu| {
            let a = cpu.register_a;
            let x = cpu.register_x;
            let and = a & x;
            let value = cpu.current_op.tmp_data;
            let result = and.wrapping_sub(value);

            cpu.set_register_x(result);
            cpu.status.set(Flags::CARRY, and >= value);
            cpu.update_zero_and_negative_flags(result);
        })
    }

    /// ARR => AND oper + ROR (Plus some wonky flag manipulation)
    pub(super) fn arr(&mut self) -> bool {
        self.exec_read_cycle(|cpu| {
            // AND
            let value = cpu.current_op.tmp_data;
            cpu.set_register_a(cpu.register_a & value);

            // ROR
            let carry = cpu.status.contains(Flags::CARRY);
            let (value, new_carry) = Self::rotate_value_right(cpu.register_a, carry);
            cpu.set_register_a(value);
            cpu.status.set(Flags::CARRY, new_carry);

            // Set carry flag based on bit 6
            cpu.status.set(Flags::CARRY, value & (1 << 6) != 0);

            // Set overflow flag based on bits 5 and 6
            let b5 = value & (1 << 5) != 0;
            let b6 = value & (1 << 6) != 0;
            cpu.status.set(Flags::OVERFLOW, b5 ^ b6);
        })
    }

    /// USBC (SBC) => SBC oper + NOP
    pub(super) fn usbc(&mut self) -> bool {
        self.sbc()
    }

    /// ANC => A AND oper, bit(7) -> C
    pub(super) fn anc(&mut self) -> bool {
        self.exec_read_cycle(|cpu| {
            let a = cpu.register_a;
            let value = cpu.current_op.tmp_data;
            cpu.set_register_a(a & value);
            cpu.status.set(Flags::CARRY, cpu.register_a & 0x80 != 0);
        })
    }

    /// ALR => AND oper + LSR
    pub(super) fn alr(&mut self) -> bool {
        self.exec_read_cycle(|cpu| {
            // AND
            let value = cpu.current_op.tmp_data;
            cpu.set_register_a(cpu.register_a & value);

            // LSR
            let carry = cpu.register_a & 1 != 0;
            cpu.status.set(Flags::CARRY, carry);
            cpu.set_register_a(cpu.register_a >> 1);
        })
    }

    /// LAS (LAR) => AND oper with SP, store in A, X, SP
    pub(super) fn las(&mut self) -> bool {
        self.exec_read_cycle(|cpu| {
            let value = cpu.current_op.tmp_data;
            let sp = cpu.stack_pointer;
            let result = sp & value;
            cpu.register_a = result;
            cpu.register_x = result;
            cpu.stack_pointer = result;
            cpu.update_zero_and_negative_flags(result);
        })
    }

    pub(super) fn jam(&mut self) -> bool {
        let opcode = self.current_op.opcode.unwrap();
        self.error = Some(CpuError::JamOpcode(opcode.code));
        true
    }

    pub(super) fn unstable(&mut self) -> bool {
        let opcode = self.current_op.opcode.unwrap();
        self.error = Some(CpuError::UnstableOpcode(opcode.code));
        true
    }
}

//////////////////
// Helpers
//////////////////
impl CPU {
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

    fn stack_pop(&mut self) -> u8 {
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        self.bus_read(CPU_STACK_BASE.wrapping_add(self.stack_pointer as u16))
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

    // fn branch(&mut self, condition: bool) {
    //     if condition {
    //         let addr = self.current_op.tmp_addr;
    //         self.set_program_counter(addr);
    //     }
    // }
}

////////////////////////////////////
// Address resolver and executors
////////////////////////////////////
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
                if self.current_op.micro_cycle == 0 {
                    self.current_op.tmp_data = self.consume_program_counter();
                    self.current_op.addr_result =
                        // AddrResult::ReadyImmediate(self.current_op.tmp_data);
                    AddrResult::ReadyImmediate
                }
            }
            AddressingMode::ZeroPage => {
                if self.current_op.micro_cycle == 0 {
                    let zero_page = self.consume_program_counter();
                    self.current_op.tmp_addr = zero_page as u16;
                    self.current_op.addr_result = AddrResult::Ready(self.current_op.tmp_addr);
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
                        self.current_op.tmp_addr =
                            self.current_op.tmp_addr.wrapping_add(index as u16) & 0x00FF;

                        self.current_op.addr_result = AddrResult::Ready(self.current_op.tmp_addr);
                    }
                    _ => {}
                }
            }
            AddressingMode::Absolute => match self.current_op.micro_cycle {
                0 => {
                    let lo = self.consume_program_counter();
                    self.current_op.tmp_addr = lo as u16;
                }
                1 => {
                    let hi = self.consume_program_counter();
                    self.current_op.tmp_addr |= (hi as u16) << 8;
                    self.current_op.addr_result = AddrResult::Ready(self.current_op.tmp_addr);
                }
                _ => {}
            },
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
                        self.current_op.base_addr = base;
                        self.current_op.tmp_addr = addr;

                        if !self.needs_dummy_cycle() {
                            self.current_op.addr_result =
                                AddrResult::Ready(self.current_op.tmp_addr);
                        }
                    }
                    2 => {
                        if self.needs_dummy_cycle() {
                            // dummy read
                            let dummy = (self.current_op.base_addr & 0xFF00)
                                | (self.current_op.tmp_addr & 0x00FF);
                            let _ = self.bus_read(dummy);
                        }
                        self.current_op.addr_result = AddrResult::Ready(self.current_op.tmp_addr);
                    }
                    _ => {}
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
                        let addr = self
                            .current_op
                            .tmp_addr
                            .wrapping_add(self.register_x as u16)
                            & 0x00FF;
                        let lo = self.bus_read(addr);
                        self.current_op.tmp_data = lo;
                        self.current_op.tmp_addr = addr;
                    }
                    3 => {
                        let hi = self.bus_read((self.current_op.tmp_addr + 1) & 0x00FF);
                        let final_addr = ((hi as u16) << 8) | self.current_op.tmp_data as u16;
                        self.current_op.tmp_addr = final_addr;
                        self.current_op.addr_result = AddrResult::Ready(self.current_op.tmp_addr);
                    }
                    _ => {}
                }
            }
            AddressingMode::IndirectY => match self.current_op.micro_cycle {
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
                    self.current_op.base_addr = base;
                    self.current_op.tmp_addr = addr;

                    if !self.needs_dummy_cycle() {
                        self.current_op.addr_result = AddrResult::Ready(self.current_op.tmp_addr);
                    }
                }
                3 => {
                    if self.needs_dummy_cycle() {
                        let dummy = (self.current_op.base_addr & 0xFF00)
                            | (self.current_op.tmp_addr & 0x00FF);
                        let _ = self.bus_read(dummy);
                    }
                    self.current_op.addr_result = AddrResult::Ready(self.current_op.tmp_addr);
                }
                _ => {}
            },
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
                        self.current_op.tmp_addr =
                            (hi as u16) << 8 | self.current_op.tmp_data as u16;
                        self.current_op.addr_result = AddrResult::Ready(self.current_op.tmp_addr);
                    }
                    _ => {}
                }
            }
            AddressingMode::Relative => {
                // Note: Branch opcodes exclusively use this address mode
                if self.current_op.micro_cycle == 0 {
                    self.current_op.tmp_data = self.consume_program_counter();
                    self.current_op.addr_result = AddrResult::Ready(self.current_op.tmp_addr);
                }
            }
            _ => unreachable!("unsupported addressing mode"),
        }
        self.current_op.micro_cycle += 1;
        self.current_op.addr_result
    }

    fn exec_read_cycle<F>(&mut self, op: F) -> bool
    where
        F: Fn(&mut CPU),
    {
        match self.tick_addressing_mode() {
            AddrResult::InProgress => false,
            AddrResult::Ready(addr) => match self.current_op.exec_phase {
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
            },
            AddrResult::ReadyImmediate => {
                op(self);
                self.current_op.exec_phase = ExecPhase::Done;
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
            AddrResult::Ready(addr) => match self.current_op.exec_phase {
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
            },
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

    fn exec_stack_pop_cycle<F>(&mut self, op: F) -> bool
    where
        F: Fn(&mut CPU),
    {
        match self.current_op.exec_phase {
            ExecPhase::Idle => {
                self.read_program_counter(); // dummy read
                self.current_op.micro_cycle += 1;
                self.current_op.exec_phase = ExecPhase::Read;
                false
            }
            ExecPhase::Read => {
                // dummy read stack
                let stack_addr = self.stack_pointer.wrapping_add(1);
                let _ = self.bus_read(CPU_STACK_BASE.wrapping_add(stack_addr as u16));

                self.current_op.micro_cycle += 1;
                self.current_op.exec_phase = ExecPhase::Write;
                false
            }
            ExecPhase::Write => {
                op(self);
                self.current_op.micro_cycle += 1;
                self.current_op.exec_phase = ExecPhase::Done;
                true
            }
            _ => unreachable!(),
        }
    }

    fn exec_stack_push_cycle<F>(&mut self, op: F) -> bool
    where
        F: Fn(&mut CPU),
    {
        match self.current_op.exec_phase {
            ExecPhase::Idle => {
                self.read_program_counter(); // dummy read
                self.current_op.micro_cycle += 1;
                self.current_op.exec_phase = ExecPhase::Write;
                false
            }
            ExecPhase::Write => {
                op(self);
                self.current_op.micro_cycle += 1;
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
        self.current_op.micro_cycle += 1;
        self.current_op.exec_phase = ExecPhase::Done;
        true
    }

    fn exec_jmp_cycle<F>(&mut self, op: F) -> bool
    where
        F: Fn(&mut CPU),
    {
        match self.tick_addressing_mode() {
            AddrResult::InProgress => false,
            AddrResult::Ready(_) => {
                op(self);
                self.current_op.exec_phase = ExecPhase::Done;
                true
            }
            AddrResult::ReadyImmediate => {
                op(self);
                self.current_op.exec_phase = ExecPhase::Done;
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
            AddrResult::Ready(_) => {
                match self.current_op.exec_phase {
                    ExecPhase::Idle => {
                        if condition(self) {
                            // branch succeed
                            self.current_op.exec_phase = ExecPhase::Internal;
                            false
                        } else {
                            // branch fail
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
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        }
    }

    pub(super) fn exec_interrupt_cycle(&mut self, interrupt: Interrupt) -> bool {
        match self.current_op.micro_cycle {
            0 => {
                if interrupt.interrupt_type == InterruptType::Nmi {
                    trace_cpu_event!(
                        "[CPU NMI ENTRY] PC={:04X} cycle={} flags=0b{:08b}",
                        self.program_counter,
                        self.cycle,
                        self.status.bits()
                    );
                }
                let _ = self.read_program_counter(); // dummy read
            }
            1 => {
                let hi = (self.program_counter >> 8) as u8;
                self.stack_push(hi);
            }
            2 => {
                let lo = self.program_counter as u8;
                self.stack_push(lo);
            }
            3 => {
                let mut status = Flags::from_bits_truncate(self.status.bits());
                status.set(
                    Flags::BREAK,
                    interrupt.b_flag_mask & Flags::BREAK.bits() != 0,
                );
                status.set(
                    Flags::BREAK2,
                    interrupt.b_flag_mask & Flags::BREAK2.bits() != 0,
                );
                self.stack_push(status.bits());
                self.status.insert(Flags::INTERRUPT_DISABLE);
            }
            4 => {
                let vector = interrupt.vector_addr;
                let lo = self.bus_read(vector);
                self.current_op.tmp_addr = lo as u16;
            }
            5 => {
                let vector = interrupt.vector_addr;
                let hi = self.bus_read(vector + 0x1);
                let lo = self.current_op.tmp_addr;
                let addr = ((hi as u16) << 8) | lo;

                self.set_program_counter(addr);
                self.current_op.exec_phase = ExecPhase::Done;
                return true;
            }
            _ => unreachable!(),
        }
        self.current_op.micro_cycle += 1;
        false
    }
}
