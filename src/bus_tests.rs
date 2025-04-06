#[cfg(test)]
mod tests {
    use crate::bus::{Bus, BusMemory};
    use crate::cartridge::nrom::NromCart;
    use crate::rom::{Mirroring, Rom};

    #[test]
    fn test_bus_fetch_and_store_byte() {
        let rom = Rom::empty();
        let cart = NromCart::new(rom.prg_rom, rom.chr_rom, rom.screen_mirroring);
        let mut bus = Bus::new(cart, |_, _| {});

        // Store a byte and verify retrieval
        bus.store_byte(5, 42);
        assert_eq!(bus.fetch_byte(5), 42);
    }

    fn setup_bus(prg_rom: Vec<u8>) -> Bus<'static> {
        let rom = Rom {
            prg_rom,
            chr_rom: vec![0; 8192],
            mapper: 0,
            screen_mirroring: Mirroring::Vertical,
        };
        let cart = NromCart::new(rom.prg_rom, rom.chr_rom, rom.screen_mirroring);
        Bus::new(cart, |_, _| {})
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
    fn test_rom_write_open_bus() {
        let mut bus = setup_bus(vec![0; 32768]);

        // With open-bus, writes to ROM do nothing
        bus.store_byte(0x8000, 0x12);
        assert_eq!(bus.fetch_byte(0x8000), 0x0);
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
        bus.ppu.write_to_ctrl(0x0);

        // Enable NMI generation
        bus.ppu.write_to_ctrl(0b1000_0000);
        for _ in 1..29781 {
            // Simulate a full frame
            bus.tick(1);
        }
        assert_eq!(bus.get_nmi_status().unwrap(), 1);
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
