use super::tracer::traceable::Traceable;
use processor::CPU;

pub mod interrupts;
pub mod opcodes;
pub mod processor;

#[cfg(test)]
pub mod processor_tests;
mod trace;

impl Traceable for CPU {
    fn trace_name(&self) -> &'static str {
        "CPU"
    }
    fn trace_state(&self) -> Option<String> {
        if self.skip_cycles == 0 {
            Some(format!(
                "(skip: {}) PC={:04X} A={:02X} X={:02X} Y={:02X} P={:02X} SP={:02X} [{:?}]",
                self.skip_cycles,
                self.program_counter,
                self.register_a,
                self.register_x,
                self.register_y,
                self.status,
                self.stack_pointer,
                self.last_opcode_desc
            ))
        } else {
            // Some(format!(
            //     "(skip: {}) PC={:04X}",
            //     self.skip_cycles, self.program_counter
            // ))
            None
        }
    }
}
