#[cfg(test)]
mod test {
    use crate::nes::cartridge::rom::Mirroring;
    use crate::nes::ppu::{PPU, PpuBusInterface};

    struct MockPpuBus {
        pub chr: [u8; 0x2000],
        pub triggered_nmi: bool,
        pub mirroring: Mirroring,
    }

    impl MockPpuBus {
        fn new() -> Self {
            Self {
                chr: [0; 0x2000],
                triggered_nmi: false,
                mirroring: Mirroring::Horizontal,
            }
        }
    }

    impl PpuBusInterface for MockPpuBus {
        fn ppu_bus_read(&mut self, addr: u16) -> u8 {
            self.chr[addr as usize % 0x2000]
        }
        fn ppu_bus_write(&mut self, addr: u16, value: u8) {
            self.chr[addr as usize % 0x2000] = value;
        }
        fn mirroring(&mut self) -> Mirroring {
            self.mirroring.clone()
        }
        // fn nmi(&mut self, _defer_one_instruction: bool) {
        //     self.triggered_nmi = true;
        // }
    }

    fn init_mock_ppu(mirroring: Mirroring) -> PPU {
        let mut ppu = PPU::new();
        let mut mock_bus = MockPpuBus::new();
        mock_bus.mirroring = mirroring;
        ppu.connect_bus(&mut mock_bus as *mut _);
        ppu
    }

    #[test]
    fn test_write_palette_table() {
        let mut ppu = init_mock_ppu(Mirroring::Horizontal);
        ppu.scroll_register.write_to_addr(0x3F);
        ppu.scroll_register.write_to_addr(0x00); // write to $3F00
        ppu.write_memory(0x34);
        assert_eq!(ppu.palette_table[0], 0x34);
    }

    #[test]
    fn test_write_palette_table_mirrored() {
        let mut ppu = init_mock_ppu(Mirroring::Vertical);
        ppu.scroll_register.write_to_addr(0x3F);
        ppu.scroll_register.write_to_addr(0x10); // write to $3F10
        ppu.write_memory(0x34);
        assert_eq!(ppu.palette_table[0], 0x34); // $3F10 mirrors down to $3F00
    }

    #[test]
    fn test_write_to_ctrl_sets_nametable_bits_in_t() {
        let mut ppu = init_mock_ppu(Mirroring::Horizontal);

        // Write to PPUCTRL with NT select bits set to 0b10
        ppu.write_register(0x2000, 0b00000010);

        // Check that t has NT bits set correctly (bits 10 and 11)
        assert_eq!(
            ppu.scroll_register.t & 0b0000110000000000,
            0b0000100000000000
        );
    }

    #[test]
    fn test_write_memory_via_registers() {
        let mut ppu = init_mock_ppu(Mirroring::Horizontal);
        let want = 0x42;

        // Set address via $2006 (high byte first, then low byte)
        ppu.write_register(0x2006, 0x21); // 0x2100 - Nametable 1
        ppu.write_register(0x2006, 0x00);

        // Write to memory via $2007
        ppu.write_register(0x2007, want);

        // Assuming horizontal mirroring: 0x2100 maps to 0x0100 in internal VRAM
        let mirrored = ppu.mirror_ram_addr(0x2100) as usize;
        assert_eq!(mirrored, 0x0100);

        // Verify VRAM
        let got = ppu.v_ram[mirrored];
        assert_eq!(got, want);
    }

    #[test]
    fn test_scroll_register_horizontal_and_vertical_write() {
        let mut ppu = init_mock_ppu(Mirroring::Horizontal);

        // First write to $2005 sets coarse X and fine X
        ppu.write_register(0x2005, 0b00110101); // value = 0x35

        assert_eq!(ppu.scroll_register.w, true);
        assert_eq!(ppu.scroll_register.t & 0b00000_11111, 6); // coarse X = 6
        assert_eq!(ppu.scroll_register.x, 0b101); // fine X = 5

        // Second write sets coarse Y and fine Y
        ppu.write_register(0x2005, 0b11010111); // 0xD7

        assert_eq!(ppu.scroll_register.w, false);
        assert_eq!((ppu.scroll_register.t >> 5) & 0b11111, 0b11010); // coarse Y = 26
        assert_eq!((ppu.scroll_register.t >> 12) & 0b111, 0b111); // fine Y = 7
    }

    #[test]
    fn test_write_to_2006_sets_t_and_v() {
        let mut ppu = init_mock_ppu(Mirroring::Horizontal);

        ppu.write_register(0x2006, 0x3F); // High byte of address
        assert_eq!(ppu.scroll_register.t, 0x3F00);
        assert_eq!(ppu.scroll_register.w, true);

        ppu.write_register(0x2006, 0x10); // Low byte
        assert_eq!(ppu.scroll_register.t, 0x3F10);
        assert_eq!(ppu.scroll_register.v, 0x3F10);
        assert_eq!(ppu.scroll_register.w, false);
    }

    #[test]
    fn test_increment_y_behavior() {
        let mut ppu = init_mock_ppu(Mirroring::Horizontal);

        // Set fine Y to 7, so it will overflow
        ppu.scroll_register.v = 0;
        ppu.scroll_register.v |= 7 << 12; // fine Y = 7
        ppu.scroll_register.v |= 5 << 5; // coarse Y = 5
        ppu.scroll_register.increment_y();

        // Should reset fine Y to 0 and increment coarse Y
        assert_eq!((ppu.scroll_register.v >> 12) & 0b111, 0); // fine Y
        assert_eq!((ppu.scroll_register.v >> 5) & 0b11111, 6); // coarse Y
    }

    #[test]
    fn test_vertical_mirroring() {
        let ppu = init_mock_ppu(Mirroring::Vertical);
        assert_eq!(ppu.mirror_ram_addr(0x2000), 0x0000); // NT0
        assert_eq!(ppu.mirror_ram_addr(0x2800), 0x0000); // NT2 -> NT0
        assert_eq!(ppu.mirror_ram_addr(0x2400), 0x0400); // NT1
        assert_eq!(ppu.mirror_ram_addr(0x2C00), 0x0400); // NT3 -> NT1
    }

    #[test]
    fn test_horizontal_mirroring() {
        let ppu = init_mock_ppu(Mirroring::Horizontal);
        assert_eq!(ppu.mirror_ram_addr(0x2000), 0x0000); // NT0
        assert_eq!(ppu.mirror_ram_addr(0x2400), 0x0000); // NT1 -> NT0
        assert_eq!(ppu.mirror_ram_addr(0x2800), 0x0400); // NT2
        assert_eq!(ppu.mirror_ram_addr(0x2C00), 0x0400); // NT3 -> NT2
    }

    #[test]
    fn test_four_screen_mirroring() {
        let ppu = init_mock_ppu(Mirroring::FourScreen);
        assert_eq!(ppu.mirror_ram_addr(0x2000), 0x0000);
        assert_eq!(ppu.mirror_ram_addr(0x2400), 0x0400);
        assert_eq!(ppu.mirror_ram_addr(0x2800), 0x0800 % 0x800); // Wraps around
        assert_eq!(ppu.mirror_ram_addr(0x2C00), 0x0C00 % 0x800); // Wraps around
    }

    #[test]
    fn test_single_screen_0() {
        let ppu = init_mock_ppu(Mirroring::Single0);
        assert_eq!(ppu.mirror_ram_addr(0x2000), 0x0000);
        assert_eq!(ppu.mirror_ram_addr(0x2400), 0x0400 % 0x400);
        assert_eq!(ppu.mirror_ram_addr(0x2800), 0x0800 % 0x400);
        assert_eq!(ppu.mirror_ram_addr(0x2C00), 0x0C00 % 0x400);
    }

    #[test]
    fn test_single_screen_1() {
        let ppu = init_mock_ppu(Mirroring::Single1);
        assert_eq!(ppu.mirror_ram_addr(0x2000), 0x0400);
        assert_eq!(ppu.mirror_ram_addr(0x2400), 0x0400);
        assert_eq!(ppu.mirror_ram_addr(0x2800), 0x0400);
        assert_eq!(ppu.mirror_ram_addr(0x2C00), 0x0400);
    }

    fn ticks_until_frame_ready(ppu: &mut PPU) -> usize {
        let mut ticks = 0usize;
        loop {
            ticks += 1;
            if ppu.tick() {
                return ticks;
            }
            assert!(ticks < 100_000, "PPU did not produce a frame in time");
        }
    }

    fn frame_ticks_with_prerender_mask(ppu: &mut PPU, enable_bg: bool) -> usize {
        let mut ticks = 0usize;
        let mask_value = if enable_bg { 0b0000_1000 } else { 0 };
        loop {
            if ppu.scanline == 261 && ppu.cycles == 338 {
                // Update $2001 late in pre-render, similar to the ROM's timing.
                ppu.write_register(0x2001, mask_value);
            }
            ticks += 1;
            if ppu.tick() {
                return ticks;
            }
            assert!(ticks < 100_000, "PPU did not produce a frame in time");
        }
    }

    #[test]
    fn test_odd_frame_skip_shortens_frame_by_one_tick() {
        let mut ppu = init_mock_ppu(Mirroring::Horizontal);

        // Enable rendering so odd-frame skip can occur.
        ppu.write_register(0x2001, 0b0000_1000);

        // Prime a couple frames; PPU starts at scanline 261 so the first "frame" is a partial pre-render line.
        ticks_until_frame_ready(&mut ppu); // partial frame
        ticks_until_frame_ready(&mut ppu); // first odd-frame

        // Check even/odd ticks
        for _ in 0..6 {
            let even_frame_ticks = ticks_until_frame_ready(&mut ppu);
            let odd_frame_ticks = ticks_until_frame_ready(&mut ppu);
            assert_eq!(even_frame_ticks, 341 * 262);
            assert_eq!(odd_frame_ticks, (341 * 262) - 1);
        }
    }

    #[test]
    fn test_even_odd_frames_patterns_match_skip_counts() {
        let mut ppu = init_mock_ppu(Mirroring::Horizontal);

        let patterns = [
            ("-----", [false, false, false, false, false], 0usize),
            ("BB---", [true, true, false, false, false], 1usize),
            ("B--B-", [true, false, false, true, false], 1usize),
            ("-B--B", [false, true, false, false, true], 1usize),
            ("BB-BB", [true, true, false, true, true], 2usize),
        ];

        ticks_until_frame_ready(&mut ppu);
        ticks_until_frame_ready(&mut ppu);
        for (label, pattern, expected_skips) in patterns {
            // Prime a couple frames; PPU starts at scanline 261 so the first "frame" is partial.

            let mut skips = 0usize;
            for enable_bg in pattern {
                let ticks = frame_ticks_with_prerender_mask(&mut ppu, enable_bg);
                if ticks == (341 * 262) - 1 {
                    skips += 1;
                }
            }

            assert_eq!(
                skips, expected_skips,
                "pattern {label} expected {expected_skips} skips, got {skips}"
            );
        }
    }
}
