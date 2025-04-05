#[cfg(test)]
mod test {
    use crate::bus::{Bus, BusMemory};
    use crate::cpu::processor::{rotate_value_left, rotate_value_right, Flags, CPU};
    use crate::rom::{Mirroring, Rom};

    fn init_cpu(prg_rom: &[u8]) -> CPU {
        let rom = Rom::new_custom(prg_rom.to_vec(), vec![], 0, Mirroring::Vertical);
        let mut bus = Bus::new(rom, |_, _| {});
        bus.enable_test_mode();
        let mut cpu = CPU::new(bus);
        cpu.program_counter = 0;
        cpu
    }

    #[test]
    fn test_0xaa_tax_0xa8_tay() {
        let program = &[
            0xa9, // LDA immediate
            0x42, //    with $0F
            0xAA, // TAX
            0xA8, // TAY
            0x02, // JAM
        ];
        let mut cpu = init_cpu(program);
        cpu.run();
        assert_eq!(cpu.register_x, 0x42);
        assert_eq!(cpu.register_y, 0x42);
        assert_eq!(cpu.status.contains(Flags::ZERO), false);
        assert_eq!(cpu.status.contains(Flags::NEGATIVE), false);
    }

    #[test]
    fn test_0xa9_lda_immediate_load_data() {
        let program = &[
            0xa9, // LDA immediate
            0x05, //    with $05
            0x02, // JAM
        ];
        let mut cpu = init_cpu(program);
        cpu.run();
        assert_eq!(cpu.register_a, 0x05);
        assert_eq!(cpu.status.contains(Flags::ZERO), false);
        assert_eq!(cpu.status.contains(Flags::NEGATIVE), false);
    }

    #[test]
    fn test_0xa9_lda_zero_flag() {
        let program = &[
            0xa9, // LDA immediate
            0x00, //    with $0
            0x02, // JAM
        ];
        let mut cpu = init_cpu(program);
        cpu.run();
        assert_eq!(cpu.status.contains(Flags::ZERO), true);
    }

    #[test]
    fn test_0xa5_lda_zero_page_load_data() {
        let program = &[
            0xa5, // LDA ZeroPage
            0x05, //    with $05
            0x02, // JAM
        ];
        let mut cpu = init_cpu(program);
        cpu.bus.store_byte(0x05, 0x42);
        cpu.run();
        assert_eq!(cpu.register_a, 0x42);
        assert_eq!(cpu.status.contains(Flags::ZERO), false);
        assert_eq!(cpu.status.contains(Flags::NEGATIVE), false);
    }

    #[test]
    fn test_0xa5_lda_zero_page_x_load_data() {
        let program = &[
            0xa9, // LDA immediate
            0x0F, //    with $0F
            0xAA, // TAX
            0xB5, // LDA ZeroPageX
            0x80, //    with $80        - X = $0F, loading A with data from $8F = 0x42
            0x02, // JAM
        ];
        let mut cpu = init_cpu(program);
        cpu.bus.store_byte(0x8F, 0x42);
        cpu.run();
        assert_eq!(cpu.register_a, 0x42);
        assert_eq!(cpu.register_x, 0x0F);
        assert_eq!(cpu.status.contains(Flags::ZERO), false);
        assert_eq!(cpu.status.contains(Flags::NEGATIVE), false);
    }

    #[test]
    fn test_0xb5_lda_absolute_load_data() {
        let program = &[
            0xAD, // LDA absolute (5 cycles)
            0xEF, //
            0xBE, // Loading from little endian $EFBE which will actually be $BEEF
            0xAA, // TAX (1 cycle)
            0x02, // JAM
        ];
        let mut cpu = init_cpu(program);
        cpu.bus.store_byte(0xBEEF, 0x42);
        cpu.run();
        assert_eq!(cpu.register_a, 0x42);
        assert_eq!(cpu.register_x, 0x42);
        assert_eq!(cpu.bus.cycles, 5 + 1);
        assert_eq!(cpu.status.contains(Flags::ZERO), false);
        assert_eq!(cpu.status.contains(Flags::NEGATIVE), false);
    }

    #[test]
    fn test_set_flags() {
        let program = &[
            0x38, // SEC
            0x78, // SEI
            0xF8, // SED
            0x02, // JAM
        ];
        let mut cpu = init_cpu(program);
        cpu.run();
        assert_eq!(cpu.status.contains(Flags::CARRY), true);
        assert_eq!(cpu.status.contains(Flags::INTERRUPT_DISABLE), true);
        assert_eq!(cpu.status.contains(Flags::DECIMAL_MODE), true);
    }

    #[test]
    fn test_set_and_clear_flags() {
        let program = &[
            0x38, // SEC
            0x78, // SEI
            0xF8, // SED
            0x18, // CLC
            0x58, // CLI
            0xD8, // CLD
            0x02, // JAM
        ];
        let mut cpu = init_cpu(program);
        cpu.run();
        assert_eq!(cpu.status.contains(Flags::CARRY), false);
        assert_eq!(cpu.status.contains(Flags::INTERRUPT_DISABLE), false);
        assert_eq!(cpu.status.contains(Flags::DECIMAL_MODE), false);
    }

    #[test]
    fn test_adc_without_carry() {
        let program = &[
            0xA9, // LDA
            0x10, //   with 0x10
            0x69, // ADC
            0x07, //   with 0x07
            0x02, // JAM
        ];
        let mut cpu = init_cpu(program);
        cpu.run();
        assert_eq!(cpu.register_a, 0x17);
        assert_eq!(cpu.status.contains(Flags::CARRY), false);
        assert_eq!(cpu.status.contains(Flags::OVERFLOW), false);
    }

    #[test]
    fn test_adc_with_overflow() {
        let program = &[
            0xA9, // LDA
            0x7F, //   with 0x7F
            0x69, // ADC
            0x0F, //   with 0x0F
            0x02, // JAM
        ];
        let mut cpu = init_cpu(program);
        cpu.run();
        assert_eq!(cpu.status.contains(Flags::CARRY), false);
        assert_eq!(cpu.status.contains(Flags::OVERFLOW), true);
        assert_eq!(cpu.register_a, 0x8E);
    }

    #[test]
    fn test_adc_with_carry() {
        let program = &[
            0xA9, // LDA
            0xFF, //   with 0xFF
            0x69, // ADC
            0x0F, //   with 0x01
            0x02, // JAM
        ];
        let mut cpu = init_cpu(program);
        cpu.run();
        assert_eq!(cpu.status.contains(Flags::CARRY), true);
        assert_eq!(cpu.status.contains(Flags::OVERFLOW), false);
        assert_eq!(cpu.register_a, 0x0E);
    }

    #[test]
    fn test_sbc_without_borrow() {
        let program = &[
            0xA9, // LDA
            0xFF, //   with 0xFF
            0x38, // SEC
            0xE9, // SBC
            0x0F, //   with 0x0F
            0x02, // JAM
        ];
        let mut cpu = init_cpu(program);
        cpu.run();
        // Note: In SBC, the "CARRY" flag becomes a "BORROW" flag which is complement
        assert_eq!(cpu.status.contains(Flags::CARRY), true);
        assert_eq!(cpu.status.contains(Flags::OVERFLOW), false);
        assert_eq!(cpu.register_a, 0xF0);
    }

    #[test]
    fn test_sbc_with_borrow() {
        let program = &[
            0xA9, // LDA
            0x00, //   with 0x00
            0x38, // SEC -- Note: it's standard to SEC before any SBC (complement of carry acts as borrow flag)
            0xE9, // SBC
            0x01, //   with 0x01
            0x02, // JAM
        ];
        let mut cpu = init_cpu(program);
        cpu.run();
        assert_eq!(cpu.status.contains(Flags::CARRY), false);
        assert_eq!(cpu.status.contains(Flags::OVERFLOW), false);
        assert_eq!(cpu.register_a, 0xFF);
    }

    #[test]
    fn test_rotate_value_right() {
        let carry = true;
        let value = 0xE0;
        let (result, new_carry) = rotate_value_right(value, carry);
        assert_eq!(result, 240);
        assert_eq!(new_carry, false);
    }

    #[test]
    fn test_rotate_value_left() {
        let carry = true;
        let value = 0xE0;
        let (result, new_carry) = rotate_value_left(value, carry);
        assert_eq!(result, 193);
        assert_eq!(new_carry, true);
    }

    #[test]
    fn test_0x8a_txa() {
        let program = &[
            0x8A, // TXA
            0x02, // JAM
        ];
        let mut cpu = init_cpu(program);
        cpu.set_register_x(0x42);
        cpu.run();
        assert_eq!(cpu.register_a, 0x42);
    }

    #[test]
    fn test_0x98_tya() {
        let program = &[
            0x98, // TYA
            0x02, // JAM
        ];
        let mut cpu = init_cpu(program);
        cpu.set_register_y(0x88);
        cpu.run();
        assert_eq!(cpu.register_a, 0x88);
    }

    #[test]
    fn test_0xba_tsx() {
        let program = &[
            0xBA, // TSX
            0x02, // JAM
        ];
        let mut cpu = init_cpu(program);
        cpu.stack_pointer = 0x37;
        cpu.run();
        assert_eq!(cpu.register_x, 0x37);
    }

    #[test]
    fn test_0x9a_txs() {
        let program = &[
            0x9A, // TXS
            0x02, // JAM
        ];
        let mut cpu = init_cpu(program);
        cpu.register_x = 0x33;
        cpu.run();
        assert_eq!(cpu.stack_pointer, 0x33);
    }

    #[test]
    fn test_0xd0_bne_success() {
        let program = &[
            0xD0, // BNE
            0x0F, //   to 0x0F
            0x02, // JAM
        ];
        let mut cpu = init_cpu(program);
        cpu.status.set(Flags::ZERO, false);
        cpu.store_byte(0x0011, 0x02); // Write BRK to branch target
        cpu.run();
        let want = 0x12;
        assert_eq!(
            cpu.program_counter,
            want,
            "{}",
            format!(
                "program_counter mismatch - Got: ${:02X} Want: ${:02X}",
                cpu.program_counter, want
            )
        );
    }

    #[test]
    fn test_0xd0_bne_failed() {
        let program = &[
            0xD0, // BNE
            0x0F, //   to 0x0F
            0x02, // JAM
        ];
        let mut cpu = init_cpu(program);
        cpu.status.set(Flags::ZERO, true);
        cpu.run();
        let want = 0x03;
        assert_eq!(
            cpu.program_counter,
            want,
            "{}",
            format!(
                "program_counter mismatch - Got: ${:02X} Want: ${:02X}",
                cpu.program_counter, want
            )
        );
    }

    #[test]
    fn test_write_to_ppu_vram() {
        let program = &[
            0xa9, 0x23, // LDA #$23    ; High byte of PPU address (0x23XX)
            0x8d, 0x06, 0x20, // STA $2006   ; Write high byte to PPUADDR
            0xa9, 0x45, // LDA #$45    ; Low byte of PPU address (0x2345)
            0x8d, 0x06, 0x20, // STA $2006   ; Write low byte to PPUADDR
            0xa9, 0x99, // LDA #$99    ; Data to write to PPU
            0x8d, 0x07, 0x20, // STA $2007   ; Store into PPUDATA
            0xa9, 0xEF, // LDA #$EF    ; Data to write to PPU
            0x8d, 0x07, 0x20, // STA $2007   ; Store into PPUDATA
            0x02, // JAM (stop execution)
        ];

        let mut prg_rom = vec![0u8; 0x4000];
        for (i, b) in program.iter().enumerate() {
            prg_rom[i] = *b;
        }

        let rom = Rom::new_custom(prg_rom, vec![], 0, Mirroring::Vertical);
        let bus = Bus::new(rom, |_, _| {});
        let mut cpu = CPU::new(bus);
        cpu.program_counter = 0x8000;
        cpu.run();

        // Verify internal PPU address register is set to $2345
        let got = cpu.bus.ppu.addr_register.get();
        let want = 0x2345 + 2; // PPU auto-increments the address after write
        assert_eq!(
            got,
            want,
            "{}",
            format!(
                "PPU addr_register incorrect. Got: ${:04X}, Want: ${:04X}",
                got, want
            )
        );

        // The VRAM address we wrote to: 0x2345 should contain 0x99
        let ppu_vram = &cpu.bus.ppu.ram;
        let mirrored_addr = 0x2345 & 0x2FFF; // VRAM mirroring
        let ram_index = mirrored_addr - 0x2000;
        assert_eq!(ppu_vram[ram_index], 0x99);
        assert_eq!(ppu_vram[ram_index + 1], 0xEF);
    }

    #[test]
    fn test_sprite_vertical_flip() {
        // This program sets up a single sprite with vertical flipping enabled
        let program = &[
            0xa9, 0x00, // LDA #$00    ; Y = 0
            0x8d, 0x03, 0x20, // STA $2003   ; Set OAMADDR to 0
            0xa9, 0x20, // LDA #$20    ; Write Y position
            0x8d, 0x04, 0x20, // STA $2004   ; Write to OAMDATA
            0xa9, 0x01, // LDA #$01    ; Tile index
            0x8d, 0x04, 0x20, // STA $2004   ; Write to OAMDATA
            0xa9, 0x80, // LDA #$80    ; Attributes: bit 7 set (vertical flip)
            0x8d, 0x04, 0x20, // STA $2004   ; Write to OAMDATA
            0xa9, 0x40, // LDA #$40    ; X = 64
            0x8d, 0x04, 0x20, // STA $2004   ; Write to OAMDATA
            0x02, // JAM (stop execution)
        ];

        let mut prg_rom = vec![0u8; 0x4000];
        for (i, b) in program.iter().enumerate() {
            prg_rom[i] = *b;
        }

        let rom = Rom::new_custom(prg_rom, vec![], 0, Mirroring::Vertical);
        let bus = Bus::new(rom, |_, _| {});
        let mut cpu = CPU::new(bus);
        cpu.program_counter = 0x8000;
        cpu.run();

        let oam = &cpu.bus.ppu.oam_data;

        // Verify sprite was written to OAM correctly
        assert_eq!(oam[0], 0x20, "Y position incorrect");
        assert_eq!(oam[1], 0x01, "Tile index incorrect");
        assert_eq!(
            oam[2], 0x80,
            "Attribute flags incorrect (should have vertical flip)"
        );
        assert_eq!(oam[3], 0x40, "X position incorrect");

        // Ensure vertical flip bit is set
        let vertical_flip = oam[2] & 0x80 != 0;
        assert!(
            vertical_flip,
            "Vertical flip bit not set in sprite attributes"
        );
    }
}
