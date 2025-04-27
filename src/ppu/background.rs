use crate::ppu::{PaletteKind, PPU};

impl PPU {
    /// `get_background_pixel` returns color-index of bg pixel at (self.cycles, self.scanline)
    pub(super) fn get_background_pixel(&mut self) -> u8 {
        debug_assert_eq!(self.mask_register.rendering_enabled(), true);
        // if self.scanline == 0 && self.cycles == 1 {
        //     let coarse_x = self.scroll_register.v & 0b00000_00000_11111;
        //     // println!("Start of frame: coarse_x = {}", coarse_x);
        // }

        // Compute bit index from fine X scroll
        let fine_x = self.scroll_register.x;
        let bit = 15 - fine_x;
        let pixel_low = (self.bg_pattern_shift_low >> bit) & 1;
        let pixel_high = (self.bg_pattern_shift_high >> bit) & 1;
        let pixel = ((pixel_high << 1) | pixel_low) as u8;

        let attr_low = (self.bg_attr_shift_low >> bit) & 1;
        let attr_high = (self.bg_attr_shift_high >> bit) & 1;
        let palette_index = ((attr_high << 1) | attr_low) as u8;

        let color = self.read_palette_color(palette_index, pixel, PaletteKind::Background);
        color
    }

    pub(super) fn load_background_registers(&mut self) {
        self.bg_pattern_shift_low =
            (self.bg_pattern_shift_low & 0xFF00) | self.next_tile_lsb as u16;
        self.bg_pattern_shift_high =
            (self.bg_pattern_shift_high & 0xFF00) | self.next_tile_msb as u16;

        // Update attribute shift registers with the latch values
        self.bg_attr_shift_low = (self.bg_attr_shift_low & 0xFF00)
            | (if self.bg_attr_latch_low != 0 {
            0xFF
        } else {
            0x00
        });
        self.bg_attr_shift_high = (self.bg_attr_shift_high & 0xFF00)
            | (if self.bg_attr_latch_high != 0 {
            0xFF
        } else {
            0x00
        });

        // Latch new values from fetched attribute byte
        self.bg_attr_latch_low = (self.next_tile_attr & 0b01) >> 0;
        self.bg_attr_latch_high = (self.next_tile_attr & 0b10) >> 1;
    }
}

impl PPU {
    pub(super) fn shift_background_registers(&mut self) {
        self.bg_pattern_shift_low <<= 1;
        self.bg_pattern_shift_high <<= 1;

        self.bg_attr_shift_low = (self.bg_attr_shift_low << 1) | self.bg_attr_latch_low as u16;
        self.bg_attr_shift_high = (self.bg_attr_shift_high << 1) | self.bg_attr_latch_high as u16;
    }

    // called during dot % 8 == 1
    pub(super) fn fetch_name_table_byte(&mut self) {
        debug_assert_eq!(self.cycles % 8, 1);
        let addr = 0x2000 | (self.scroll_register.v & 0x0FFF);
        self.next_tile_id = self.read_bus(addr);
    }

    // called during dot % 8 == 3
    pub(super) fn fetch_attribute_byte(&mut self) {
        debug_assert_eq!(self.cycles % 8, 3);
        let v = self.scroll_register.v;

        let addr = 0x23C0
            | (v & 0x0C00)            // nametable select (bits 10–11 of v)
            | ((v >> 4) & 0b111_000)  // (coarse_y / 4) << 3
            | ((v >> 2) & 0b000_111); // (coarse_x / 4)
        let attr_byte = self.read_bus(addr);

        // Extract coarse X/Y positions from v
        let coarse_x = (v >> 0) & 0b11111;
        let coarse_y = (v >> 5) & 0b11111;

        // Determine which quadrant within the attribute byte
        let shift = ((coarse_y & 0x02) << 1) | (coarse_x & 0x02);
        self.next_tile_attr = (attr_byte >> shift) & 0b11;
    }

    // called during dot % 8 == 5
    pub(super) fn fetch_tile_low_byte(&mut self) {
        debug_assert_eq!(self.cycles % 8, 5);
        let fine_y = (self.scroll_register.v >> 12) & 0b111;
        let base = self.ctrl_register.background_pattern_addr();
        let tile_addr = base + (self.next_tile_id as u16) * 16 + fine_y;
        self.next_tile_lsb = self.read_bus(tile_addr);
    }

    // called during dot % 8 == 7
    pub(super) fn fetch_tile_high_byte(&mut self) {
        debug_assert_eq!(self.cycles % 8, 7);
        let fine_y = (self.scroll_register.v >> 12) & 0b111;
        let base = self.ctrl_register.background_pattern_addr();
        let tile_addr = base + (self.next_tile_id as u16) * 16 + fine_y + 8;
        self.next_tile_msb = self.read_bus(tile_addr);
    }
}