use crate::nes::cpu::{CPU, CpuBusInterface};

pub struct SimpleBus {
    pub cpu_ram: [u8; 0x10000],
    pub cpu: CPU,
    pub cycles: usize,
}

impl SimpleBus {
    pub fn new(program: Vec<u8>) -> SimpleBus {
        let mut bus = SimpleBus {
            cpu_ram: [0; 0x10000],
            cpu: CPU::new(),
            cycles: 0,
        };
        for i in 0..program.len() {
            bus.cpu_ram[i] = program[i];
        }
        bus
    }

    pub fn reset(&mut self) {
        self.cpu.reset();
        self.cycles = 0;
        self.cpu.program_counter = 0x0000;
    }

    pub fn tick(&mut self) -> (bool, bool) {
        let (is_done, is_breaking) = self.cpu.tick();
        self.cycles += 1;
        (is_done, is_breaking)
    }
}

impl CpuBusInterface for SimpleBus {
    fn cpu_bus_read(&mut self, addr: u16) -> u8 {
        self.cpu_ram[addr as usize]
    }
    fn cpu_bus_write(&mut self, addr: u16, value: u8) {
        self.cpu_ram[addr as usize] = value;
    }
}
