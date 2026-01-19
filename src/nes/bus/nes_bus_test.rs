#[cfg(test)]
mod test {
    use crate::nes::bus::nes_bus::NesBus;
    use crate::nes::cartridge::rom::{Mirroring, Rom};

    #[test]
    fn test_cpu_write_to_nametables_via_2006_2007() {
        let prg_rom = vec![0; 0x4000];
        let chr_rom = vec![0; 0x2000];
        let mapper = 0;
        let screen_mirroring = Mirroring::Vertical;
        let rom = Rom::new_custom(prg_rom, chr_rom, mapper, screen_mirroring);
        let cartridge = rom.into_cartridge().unwrap();
        let bus = NesBus::new_with_cartridge(cartridge);

        // Test a handful of addresses within $2000-$2FFF
        let test_addresses = [0x2000, 0x2400, 0x27FF, 0x2C00, 0x2FFF];

        for &addr in &test_addresses {
            let value = (addr & 0xFF) as u8;

            // CPU writes to $2006 to set high byte, then low byte of VRAM address
            let high = (addr >> 8) as u8;
            let low = (addr & 0xFF) as u8;
            bus.cpu.bus_write(0x2006, high);
            bus.cpu.bus_write(0x2006, low);

            // Then writes to $2007 to store the value at that address
            bus.cpu.bus_write(0x2007, value);

            // Now confirm that value landed at the mirrored VRAM address
            let mirrored = bus.ppu.mirror_ram_addr(addr);
            let actual = bus.ppu.v_ram[mirrored as usize];

            assert_eq!(
                actual, value,
                "VRAM write failed: CPU wrote {:02X} to ${:04X} (mirrored to ${:04X}) but found {:02X}",
                value, addr, mirrored, actual
            );
        }
    }

    #[test]
    fn test_cpu_write_auto_increment_vram_address() {
        let prg_rom = vec![0; 0x4000];
        let chr_rom = vec![0; 0x2000];
        let mapper = 0;
        let screen_mirroring = Mirroring::Vertical;
        let rom = Rom::new_custom(prg_rom, chr_rom, mapper, screen_mirroring);
        let cartridge = rom.into_cartridge().unwrap();
        let bus = NesBus::new_with_cartridge(cartridge);

        // Set increment mode to 1 (bit 2 = 0 increments by 1 after each $2007 write)
        // Write to $2000 (PPUCTRL)
        bus.cpu.bus_write(0x2000, 0x00); // increment by 1 (not 32)

        let start_addr = 0x2400;
        let values = [0xAA, 0xBB, 0xCC, 0xDD];

        // Set VRAM address via $2006
        let high = (start_addr >> 8) as u8;
        let low = (start_addr & 0xFF) as u8;
        bus.cpu.bus_write(0x2006, high);
        bus.cpu.bus_write(0x2006, low);

        // Write each value via $2007
        for (i, &val) in values.iter().enumerate() {
            bus.cpu.bus_write(0x2007, val);

            // Determine where write should have landed
            let expected_addr = start_addr + i as u16;
            let mirrored = bus.ppu.mirror_ram_addr(expected_addr);
            let actual = bus.ppu.v_ram[mirrored as usize];

            assert_eq!(
                actual, val,
                "Auto-increment failed at write {}: expected {:02X} at ${:04X} (mirrored ${:04X}), found {:02X}",
                i, val, expected_addr, mirrored, actual
            );
        }
    }

    #[test]
    fn test_cpu_write_across_frames_waiting_for_vblank() {
        let prg_rom = vec![0; 0x4000];
        let chr_rom = vec![0; 0x2000];
        let mapper = 0;
        let screen_mirroring = Mirroring::Vertical;
        let rom = Rom::new_custom(prg_rom, chr_rom, mapper, screen_mirroring);
        let cartridge = rom.into_cartridge().unwrap();
        let bus = NesBus::new_with_cartridge(cartridge);

        // Set increment mode to 1 (increment by 1 after each write)
        bus.cpu.bus_write(0x2000, 0x00);

        let start_addr = 0x2400;
        let values = [0xAA, 0xBB, 0xCC, 0xDD];

        // Set VRAM address via $2006 (hi then lo)
        let high = (start_addr >> 8) as u8;
        let low = (start_addr & 0xFF) as u8;

        // Wait until VBlank starts
        while !(bus.ppu.scanline == 241 && bus.ppu.cycles == 0) {
            bus.ppu.tick();
        }
        bus.cpu.bus_write(0x2006, high);
        for i in 0..3 {
            bus.ppu.tick();
        }
        bus.cpu.bus_write(0x2006, low);
        for i in 0..3 {
            bus.ppu.tick();
        }

        // Write values one at a time across frames
        for (i, &val) in values.iter().enumerate() {
            for i in 0..3 {
                bus.ppu.tick();
            }

            bus.cpu.bus_write(0x2007, val);
            println!(
                "VLBANK: i={} ppu.scanline={} ppu.cycles={}\tppu.scroll_register.v=${:04X} data={:02X}",
                i, bus.ppu.scanline, bus.ppu.cycles, bus.ppu.scroll_register.v, val
            );

            let expected_addr = start_addr + i as u16;
            let mirrored = bus.ppu.mirror_ram_addr(expected_addr);
            let actual = bus.ppu.v_ram[mirrored as usize];

            assert_eq!(
                actual, val,
                "Write across frame {} failed: expected {:02X} at ${:04X} (mirrored ${:04X}), got {:02X}",
                i, val, expected_addr, mirrored, actual
            );
        }
    }

    #[test]
    fn test_cpu_read_vram_with_read_buffer() {
        let prg_rom = vec![0; 0x4000];
        let chr_rom = vec![0; 0x2000];
        let mapper = 0;
        let screen_mirroring = Mirroring::Vertical;
        let rom = Rom::new_custom(prg_rom, chr_rom, mapper, screen_mirroring);
        let cartridge = rom.into_cartridge().unwrap();
        let bus = NesBus::new_with_cartridge(cartridge);

        // Set increment mode to 1 (increment by 1)
        bus.cpu.bus_write(0x2000, 0x00);

        // Directly write to VRAM, bypassing CPU
        let base_addr = 0x2400;
        let mirrored = bus.ppu.mirror_ram_addr(base_addr);
        bus.ppu.v_ram[mirrored as usize] = 0xDE;

        // Set PPU address via $2006
        let high = (base_addr >> 8) as u8;
        let low = (base_addr & 0xFF) as u8;
        bus.cpu.bus_write(0x2006, high);
        bus.cpu.bus_write(0x2006, low);

        // First read returns buffer contents (invalid)
        let dummy = bus.cpu.bus_read(0x2007);

        // Second read returns actual value at 0x2400 (0xDE)
        let value = bus.cpu.bus_read(0x2007);

        assert_ne!(
            dummy, 0xDE,
            "First read from $2007 should not return actual VRAM value (buffer delay)."
        );

        assert_eq!(
            value, 0xDE,
            "Second read from $2007 should return value from VRAM at 0x2400."
        );
    }

    #[test]
    fn test_write_to_ppu_vram() {
        let program = &[
            0xa9, 0x23, // LDA #$23
            0x8d, 0x06, 0x20, // STA $2006
            0xa9, 0x45, // LDA #$45
            0x8d, 0x06, 0x20, // STA $2006 (PPU addr_ptr = $2345)
            0xa9, 0x99, // LDA #$99
            0x8d, 0x07, 0x20, // STA $2007 ([$2345] = $99; addr_ptr = $2346))
            0xa9, 0xEF, // LDA #$EF
            0x8d, 0x07, 0x20, // STA $2007 ([$2346] = $EF; addr_ptr = $2347)
            0xa9, 0x3F, // LDA #$3F
            0x8d, 0x06, 0x20, // STA $2006
            0xa9, 0x00, // LDA #$00
            0x8d, 0x06, 0x20, // STA $2006
            0xa9, 0x42, // LDA #$42
            0x8d, 0x07, 0x20, // STA $2007 ([$3F00] = $42; addr_ptr increased to $3F01)
            0x02, // JAM
        ];

        let mut prg_rom = vec![0u8; 0x8000];
        prg_rom[..program.len()].copy_from_slice(program);

        let chr_rom = vec![0u8; 0x2000];
        let rom = Rom::new_custom(prg_rom, chr_rom, 0, Mirroring::Vertical);
        let cartridge = rom.into_cartridge().unwrap();
        let bus = NesBus::new_with_cartridge(cartridge);
        bus.cpu.program_counter = 0x8000;

        // Fast-forward PPU to VBlank
        while !(bus.ppu.scanline == 241 && bus.ppu.cycles == 0) {
            bus.ppu.tick();
        }

        // Execute instructions until JAM
        while !bus.cpu.tick().1 {}

        let addr = 0x2345;
        let mirror = bus.ppu.mirror_ram_addr(addr) as usize;
        assert_eq!(bus.ppu.v_ram[mirror + 0], 0x99);
        assert_eq!(bus.ppu.v_ram[mirror + 1], 0xEF);

        // Verify internal PPU address register is set to $3F01
        let got = bus.ppu.scroll_register.get_addr();
        let want = 0x3F00 + 1; // PPU auto-increments the address after write
        assert_eq!(
            got, want,
            "PPU addr_register incorrect. Got: ${:04X}, Want: ${:04X}",
            got, want
        );
    }

    #[test]
    fn test_write_and_read_of_ppu_palette_data() {
        let program = &[
            0xa9, 0x3F, // LDA #$3F
            0x8d, 0x06, 0x20, // STA $2006
            0xa9, 0x00, // LDA #$00
            0x8d, 0x06, 0x20, // STA $2006 (addr_ptr = $3F00))
            0xa9, 0x42, // LDA #$42
            0x8d, 0x07, 0x20, // STA $2007 ([$3F00] = $42; addr_ptr = $3F01))
            0xa9, 0x84, // LDA #$84
            0x8d, 0x07, 0x20, // STA $2007 ([$3F01] = $84; addr_ptr = $3F02))
            0x02, // JAM
        ];

        let mut prg_rom = vec![0u8; 0x8000];
        prg_rom[..program.len()].copy_from_slice(program);

        let chr_rom = vec![0u8; 0x2000];
        let rom = Rom::new_custom(prg_rom, chr_rom, 0, Mirroring::Vertical);
        let cartridge = rom.into_cartridge().unwrap();
        let bus = NesBus::new_with_cartridge(cartridge);
        bus.cpu.program_counter = 0x8000;

        // Fast-forward to VBLANK
        bus.ppu.run_until_vblank();

        // Execute instructions until JAM
        while !bus.cpu.tick().1 {}

        // Verify palette_table was written to
        let palette_base = 0x3F00;
        let addr = 0x3F00;
        assert_eq!(bus.ppu.palette_table[(addr - palette_base) + 0], 0x42);
        assert_eq!(bus.ppu.palette_table[(addr - palette_base) + 1], 0x84);

        // Verify internal PPU address register is set to $3F02
        let got = bus.ppu.scroll_register.get_addr();
        let want = 0x3F00 + 2; // PPU auto-increments the address after write
        assert_eq!(
            got, want,
            "PPU addr_register incorrect. Got: ${:04X}, Want: ${:04X}",
            got, want
        );

        assert_eq!(bus.ppu.scroll_register.w, false); // Verify ScrollRegister's latch is reset
    }
}
