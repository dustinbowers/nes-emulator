use crate::memory::heap_memory::HeapMemory;
use crate::memory::memory_trait::MemoryTrait;
use crate::ppu::PPU;
use crate::rom::Rom;

const CPU_RAM_SIZE: usize = 2048;
const CPU_RAM_START: u16 = 0x0000;
const CPU_RAM_END: u16 = 0x1FFF;
const CPU_MIRROR_MASK: u16 = 0b0000_0111_1111_1111;

const PPU_REGISTERS_START: u16 = 0x2000;
const PPU_REGISTERS_END: u16 = 0x3FFF;
const PPU_MIRROR_MASK: u16 = 0b0010_0000_0000_0111;
const ROM_START: u16 = 0x8000;
const ROM_END: u16 = 0xFFFF;

pub struct Bus {
    pub cpu_ram: HeapMemory<u8>,
    pub cycles: usize,
    pub prg_rom: Vec<u8>,
    pub ppu: PPU,
    pub disable_mirroring: bool,
    pub ready_to_render: bool,

    // Some games expect an "open-bus": When reading from invalid addresses,
    // the bus should return its last-read value
    pub last_fetched_byte: u8,
}

pub trait BusMemory {
    type DisableMirroring;
    fn fetch_byte(&mut self, address: u16) -> u8;
    fn store_byte(&mut self, address: u16, value: u8);

    fn fetch_u16(&mut self, address: u16) -> u16 {
        let lo = self.fetch_byte(address) as u16;
        let hi = self.fetch_byte(address.wrapping_add(1)) as u16;
        hi << 8 | lo
    }

    fn store_u16(&mut self, address: u16, value: u16) {
        self.store_byte(address, (value >> 8) as u8);
        self.store_byte(address.wrapping_add(1), value as u8);
    }
}

impl BusMemory for Bus {
    type DisableMirroring = bool;

    fn fetch_byte(&mut self, address: u16) -> u8 {
        if self.disable_mirroring {
            return *self.cpu_ram.read(address as usize);
        }
        let fetched_byte = match address {
            CPU_RAM_START..=CPU_RAM_END => {
                let mirrored_address = address & CPU_MIRROR_MASK;
                *self.cpu_ram.read(mirrored_address as usize)
            }
            PPU_REGISTERS_START..=PPU_REGISTERS_END => {
                let mirrored_address = address & PPU_MIRROR_MASK;
                match mirrored_address {
                    0x2002 => self.ppu.read_status(),
                    0x2004 => self.ppu.read_oam_data(),
                    0x2007 => self.ppu.read_data(),
                    _ => {
                        println!(
                            "Attempt to read from write-only PPU register ${:04X}",
                            address
                        );
                        self.last_fetched_byte
                    }
                }
            }
            ROM_START..=ROM_END => self.read_prg_rom(address),

            0x4000..=0x4015 => {
                // ignore APU
                0
            }
            0x4016 => {
                // TODO: implement joypad 1
                0
            }
            0x4017 => {
                // ignore joypad 2 for now
                0
            }
            _ => {
                println!("Invalid fetch from ${:04X}", address);
                self.last_fetched_byte
            }
        };
        self.last_fetched_byte = fetched_byte;
        fetched_byte
    }
    fn store_byte(&mut self, address: u16, value: u8) {
        if self.disable_mirroring {
            self.cpu_ram.write(address as usize, value);
            return;
        }

        println!("bus.store_byte(${:04X}, ${:02X})", address, value);
        match address {
            CPU_RAM_START..=CPU_RAM_END => {
                let mirrored_address = address & CPU_MIRROR_MASK;
                println!("writing to CPU_RAM ${:04X}, mirrored to: ${:04X}", address, mirrored_address);
                self.cpu_ram.write(mirrored_address as usize, value);
            }
            PPU_REGISTERS_START..=PPU_REGISTERS_END => {
                let mirror_down_address = address & 0b0010_0000_0000_0111;
                println!("writing to PPU ${:04X}, mirrored to: ${:04X}", address, mirror_down_address);
                match mirror_down_address {
                    0x2000 => self.ppu.write_to_ctrl(value),
                    0x2001 => self.ppu.write_to_mask(value),
                    0x2002 => panic!("attempt to write to PPU status register"),
                    0x2003 => self.ppu.set_oam_addr(value),
                    0x2004 => self.ppu.write_to_oam_data(value),
                    0x2005 => self.ppu.write_to_scroll(value),
                    0x2006 => self.ppu.set_ppu_addr(value),
                    0x2007 => self.ppu.write_to_data(value),
                    _ => panic!("Invalid mirrored PPU register write: ${:04X}", address),
                }
            }
            0x4000..=0x4013 | 0x4015 => {
                // TODO: implement APU
            }
            ROM_START..=ROM_END => {
                panic!("{}", format!("Attempted write to ROM! (${:04X})", address))
            }

            0x4014 => {
                let hi: u16 = (value as u16) << 8;
                let mut buffer: [u8; 256] = [0; 256];

                for i in 0..256 {
                    buffer[i] = self.fetch_byte(hi + i as u16);
                }

                self.ppu.write_to_oam_dma(&buffer);
                // TODO: NES pauses CPU for 512 cycles during DMA
            }
            0x4016 => {
                // TODO: implement joypad 1
            }
            0x4017 => {
                // ignore joypad 2
            }
            _ => {
                println!(
                    "Unhandled write to ${:04X} with value ${:02X}",
                    address, value
                );
            }
        }
    }
}

impl Bus {
    pub fn new(rom: Rom) -> Self {
        let prg_rom = rom.prg_rom.clone();
        let ppu = PPU::new(rom.chr_rom, rom.screen_mirroring);

        Self {
            cpu_ram: HeapMemory::new(CPU_RAM_SIZE, 0u8),
            cycles: 0,
            prg_rom,
            ppu,
            disable_mirroring: false,
            ready_to_render: false,
            last_fetched_byte: 0,
        }
    }

    pub fn enable_test_mode(&mut self) {
        self.disable_mirroring = true;
        self.cpu_ram.data = std::mem::take(&mut self.prg_rom.clone());
        self.cpu_ram.data.resize(1 << 16, 0u8);
    }

    pub fn tick(&mut self, cycles: usize) {
        self.cycles += cycles;

        let pre_nmi = self.ppu.get_nmi_status();
        self.ppu.tick(cycles * 3);
        let post_nmi = self.ppu.get_nmi_status();
        if !pre_nmi && post_nmi {
            self.ready_to_render = true
        }
    }

    pub fn get_nmi_status(&mut self) -> bool {
        self.ppu.get_nmi_status()
    }

    pub fn fetch_bytes(&mut self, address: u16, size: u8) -> &[u8] {
        self.cpu_ram.read_n(address as usize, size as usize)
    }

    pub fn fetch_bytes_raw(&mut self, address: u16, size: u16) -> &[u8] {
        self.cpu_ram.read_n(address as usize, size as usize)
    }

    pub fn store_bytes(&mut self, address: u16, values: &[u8]) {
        self.cpu_ram.write_n(address as usize, values);
    }

    pub fn store_byte_vec(&mut self, address: u16, values: Vec<u8>) {
        self.cpu_ram
            .write_n(address as usize, &values.into_boxed_slice())
    }

    fn read_prg_rom(&self, addr: u16) -> u8 {
        let addr = addr - 0x8000;

        // Calculate the effective address with mirroring if needed
        let effective_addr = if self.prg_rom.len() == 0x4000 && addr >= 0x4000 {
            addr % 0x4000
        } else {
            addr
        };
        self.prg_rom[effective_addr as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rom::{Mirroring, Rom};

    #[test]
    fn test_bus_fetch_and_store_byte() {
        let rom = Rom::empty();
        let mut bus = Bus::new(rom);

        // Store a byte and verify retrieval
        bus.store_byte(5, 42);
        assert_eq!(bus.fetch_byte(5), 42);
    }

    fn setup_bus(prg_rom: Vec<u8>) -> Bus {
        let rom = Rom {
            prg_rom,
            chr_rom: vec![0; 8192],
            mapper: 0,
            screen_mirroring: Mirroring::Vertical,
        };
        Bus::new(rom)
    }

    #[test]
    fn test_cpu_ram_read_write() {
        let mut bus = setup_bus(vec![0; 32768]);

        let addr = 0x0004;
        bus.store_byte(addr, 0x42);
        assert_eq!(bus.fetch_byte(addr), 0x42);
    }

    #[test]
    fn test_cpu_ram_mirroring() {
        let mut bus = setup_bus(vec![0; 32768]);

        let addr = 0x0002;
        let mirrored_addr = addr | 0x0800; // Mirrors in range 0x0000-0x1FFF
        bus.store_byte(addr, 0x55);
        assert_eq!(bus.fetch_byte(mirrored_addr), 0x55);
    }

    #[test]
    #[should_panic(expected = "Attempted write to ROM!")]
    fn test_rom_write_panics() {
        let mut bus = setup_bus(vec![0; 32768]);

        bus.store_byte(0x8000, 0x12); // Should panic
    }

    #[test]
    fn test_rom_read() {
        let mut bus = setup_bus(vec![0xAA; 32768]);

        assert_eq!(bus.fetch_byte(0x8000), 0xAA);
    }

    #[test]
    fn test_prg_rom_mirroring() {
        let mut bus = setup_bus(vec![0xCC; 16384]); // 16K PRG-ROM, should mirror

        assert_eq!(bus.fetch_byte(0x8000), 0xCC);
        assert_eq!(bus.fetch_byte(0xC000), 0xCC); // Mirrored region
    }

    #[test]
    fn test_ppu_register_read_write() {
        let mut bus = setup_bus(vec![0; 32768]);

        bus.store_byte(0x2000, 0xFF); // Write to PPU CTRL
        // Since 0x2000 is write-only, we cannot verify by reading, but ensure no crash occurs.
    }

    #[test]
    fn test_open_bus_behavior() {
        let mut bus = setup_bus(vec![0; 32768]);

        bus.last_fetched_byte = 0xAB;
        assert_eq!(bus.fetch_byte(0x5000), 0xAB);
    }

    #[test]
    fn test_vram_increment() {
        let mut bus = setup_bus(vec![0; 32768]);

        bus.ppu.write_to_ctrl(0b0000_0100);
        // bus.ppu.write_to_ctrl(ControlRegister::VRAM_ADD_INCREMENT.bits());
        assert_eq!(bus.ppu.ctrl_register.increment_ram_addr(), 32);
    }

    #[test]
    fn test_uninitialized_memory_reads_return_open_bus_value() {
        let mut bus = setup_bus(vec![0; 32768]);

        bus.last_fetched_byte = 0xBE;
        assert_eq!(bus.fetch_byte(0x5000), 0xBE); // Open-bus behavior
    }

    #[test]
    fn test_mirrored_cpu_ram_access() {
        let mut bus = setup_bus(vec![0; 32768]);

        let base_addr = 0x0001;
        let mirror_addr = base_addr | 0x0800; // Mirrored in 0x0000-0x1FFF

        bus.store_byte(base_addr, 0x37);
        assert_eq!(bus.fetch_byte(mirror_addr), 0x37);
    }

    #[test]
    fn test_rom_read_correctness() {
        let mut bus = setup_bus(vec![0xDE, 0xAD, 0xBE, 0xEF]); // ROM contains known bytes

        assert_eq!(bus.fetch_byte(0x8000), 0xDE);
        assert_eq!(bus.fetch_byte(0x8001), 0xAD);
        assert_eq!(bus.fetch_byte(0x8002), 0xBE);
        assert_eq!(bus.fetch_byte(0x8003), 0xEF);
    }

    #[test]
    fn test_prg_rom_16k_mirroring() {
        let mut bus = setup_bus(vec![0x99; 16384]); // 16KB PRG-ROM

        assert_eq!(bus.fetch_byte(0x8000), 0x99);
        assert_eq!(bus.fetch_byte(0xC000), 0x99); // Mirrored in 16KB banks
    }

    #[test]
    fn test_ppu_register_mirroring() {
        let mut bus = setup_bus(vec![0; 32768]);

        bus.store_byte(0x2000, 0x80); // PPU Control
        bus.store_byte(0x2001, 0x40); // PPU Mask

        assert_eq!(bus.ppu.ctrl_register.bits(), 0x80);
        assert_eq!(bus.ppu.mask_register.bits(), 0x40);

        // PPU registers are mirrored every 8 bytes
        bus.store_byte(0x2008, 0x33);
        assert_eq!(bus.ppu.ctrl_register.bits(), 0x33);
    }

    #[test]
    fn test_oam_dma_transfer() {
        let mut bus = setup_bus(vec![0; 32768]);

        // Prepare a fake page of data in CPU RAM
        let base_address = 0x0300;
        let data = [0xAB; 256];
        bus.store_bytes(base_address, &data);

        // Perform DMA transfer from CPU RAM to OAM
        bus.store_byte(0x4014, (base_address >> 8) as u8);

        assert_eq!(bus.ppu.oam_data[0], 0xAB);
        assert_eq!(bus.ppu.oam_data[255], 0xAB);
    }

    #[test]
    fn test_vblank_nmi_triggering() {
        let mut bus = setup_bus(vec![0; 32768]);

        // Disable NMI initially
        bus.ppu.write_to_ctrl(0x00);
        assert_eq!(bus.ppu.get_nmi_status(), false);

        // Enable NMI generation
        bus.ppu.write_to_ctrl(0b1000_0000);
        for i in 1..29781 { // Simulate a full frame
            bus.tick(1);
        }
        assert_eq!(bus.get_nmi_status(), true);
    }

    #[test]
    fn test_read_modify_write_behavior() {
        let mut bus = setup_bus(vec![0; 32768]);

        let addr = 0x0005;
        bus.store_byte(addr, 0x12);
        let value = bus.fetch_byte(addr);
        bus.store_byte(addr, value.wrapping_add(1));

        assert_eq!(bus.fetch_byte(addr), 0x13);
    }

    #[test]
    fn test_multiple_rom_banks_access() {
        let mut rom_data = vec![0x00; 32768];
        rom_data[0] = 0xAA;
        rom_data[0x4000] = 0xBB;

        let mut bus = setup_bus(rom_data);

        assert_eq!(bus.fetch_byte(0x8000), 0xAA);
        assert_eq!(bus.fetch_byte(0xC000), 0xBB);
    }
}
