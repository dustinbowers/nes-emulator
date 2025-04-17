use crate::cartridge::Cartridge;
use crate::controller::NesController;
use crate::cpu::processor::CpuBusInterface;
use crate::ppu::PpuBusInterface;

pub mod nes_bus;
pub mod simple_bus;

#[cfg(test)]
mod nes_bus_test;
