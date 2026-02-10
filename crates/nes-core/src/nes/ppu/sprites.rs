use super::PPU;
impl PPU {
    /// get_sprite_pixel determines the sprite pixel.
    /// Returns (sprite_palette, sprite_pixel, sprite_behind_bg, sprite_zero_rendered)
    pub(super) fn get_sprite_pixel(&mut self) -> (u8, u8, bool, bool) {
        let mut sprite_palette = 0;
        let mut sprite_pixel = 0;
        let mut sprite_in_front = false; // true = behind background (OAM bit 5 = 1)
        let mut sprite_zero_rendered = false;
        let left_clip_enabled = !self.mask_register.leftmost_8pxl_sprite();

        for i in 0..8 {
            if self.sprite_x_counter[i] == 0 {
                let in_left_clip = left_clip_enabled && self.cycles <= 8;
                let low_bit = (self.sprite_pattern_low[i] >> 7) & 1;
                let high_bit = (self.sprite_pattern_high[i] >> 7) & 1;
                let pixel = (high_bit << 1) | low_bit;

                if pixel != 0 && !in_left_clip {
                    // Record sprite-0 hit candidate regardless of priority
                    if i == 0 && self.sprite_zero_in_range {
                        sprite_zero_rendered = true;
                    }

                    // Only take the first opaque sprite pixel
                    if sprite_pixel == 0 {
                        let palette = self.sprite_attributes[i] & 0b11;
                        sprite_palette = palette;
                        sprite_pixel = pixel;

                        // Bit 5 = 1 means behind background
                        sprite_in_front = (self.sprite_attributes[i] & 0b0010_0000) == 0;
                    }
                }
            }
        }

        (
            sprite_palette,
            sprite_pixel,
            sprite_in_front,
            sprite_zero_rendered,
        )
    }

    pub(super) fn sprite_evaluation(&mut self, scanline: usize, dot: usize) {
        let current_sprite_index = (dot - 65) / 2;
        if current_sprite_index >= 64 {
            return;
        }
        debug_assert!(dot % 2 == 1); // Only runs on odd cycles

        let oam_base = 4 * current_sprite_index;
        let secondary_oam_base = self.sprite_count * 4; // sprite_count ranges 0..8

        let current_oam_y = self.oam_data[oam_base] as usize;
        let sprite_height = self.ctrl_register.sprite_size() as i16;
        let render_scanline = scanline + 1;

        if current_oam_y < 0xFF
            && render_scanline >= current_oam_y + 1
            && render_scanline < (current_oam_y + 1 + sprite_height as usize)
        {
            if self.sprite_count < 8 {
                self.secondary_oam[secondary_oam_base] = self.oam_data[oam_base];
                self.secondary_oam[secondary_oam_base + 1] = self.oam_data[oam_base + 1];
                self.secondary_oam[secondary_oam_base + 2] = self.oam_data[oam_base + 2];
                self.secondary_oam[secondary_oam_base + 3] = self.oam_data[oam_base + 3];

                // Check if sprite0 is in range
                if current_sprite_index == 0 {
                    self.sprite_zero_in_range_next = true; // Look out for possible sprite0 hit
                }

                self.sprite_count += 1;
            } else {
                // Too many sprites!
                self.status_register.set_sprite_overflow(true);
            }
        }
    }

    pub(super) fn sprite_fill_register(&mut self, sprite_num: usize, scanline: usize) {
        let base = 4 * sprite_num;

        let y = self.secondary_oam[base];
        let tile_index = self.secondary_oam[base + 1];
        let attributes = self.secondary_oam[base + 2];
        let x = self.secondary_oam[base + 3];

        let sprite_height = self.ctrl_register.sprite_size() as i16;
        let render_scanline = scanline + 1;
        let mut row = (render_scanline as i16) - (y as i16 + 1);

        // Vertical flip
        if attributes & 0x80 != 0 {
            row = (sprite_height - 1) - row;
        }

        let pattern_addr = if sprite_height == 16 {
            // 8x16
            let table = (tile_index & 0x01) as u16;
            let tile_num = (tile_index & 0xFE) as u16;
            let fine_y = (row as u16) & 0x07;
            let tile_offset = if row < 8 { 0 } else { 1 };

            (table * 0x1000) + ((tile_num + tile_offset) * 16) + fine_y
        } else {
            // 8x8
            let table_addr = self.ctrl_register.sprite_pattern_addr();
            let fine_y = (row as u16) & 0x07;
            table_addr + (tile_index as u16) * 16 + fine_y
        };

        // Always run sprite pattern fetches to keep mapper A12 timing accurate
        let mut pattern_low = self.read_bus(pattern_addr);
        let mut pattern_high = self.read_bus(pattern_addr + 8);

        // Horizontal flip
        let horizontal_flip = (attributes & 0x40) != 0;
        if horizontal_flip {
            pattern_low = pattern_low.reverse_bits();
            pattern_high = pattern_high.reverse_bits();
        }

        if sprite_num < self.sprite_count {
            self.sprite_x_counter[sprite_num] = x;
            self.sprite_x_latch[sprite_num] = x;
            self.sprite_attributes[sprite_num] = attributes;
            self.sprite_pattern_low[sprite_num] = pattern_low;
            self.sprite_pattern_high[sprite_num] = pattern_high;
        } else {
            // Clear unused sprite slots
            self.sprite_x_counter[sprite_num] = 0xFF; // Off-screen
            self.sprite_x_latch[sprite_num] = 0xFF;
            self.sprite_attributes[sprite_num] = 0;
            self.sprite_pattern_low[sprite_num] = 0;
            self.sprite_pattern_high[sprite_num] = 0;
        }
    }

    pub(super) fn shift_sprite_registers(&mut self) {
        // Shift sprite pattern registers for next pixel
        for i in 0..8 {
            if self.sprite_x_counter[i] > 0 {
                self.sprite_x_counter[i] -= 1;
            } else {
                self.sprite_pattern_low[i] <<= 1;
                self.sprite_pattern_high[i] <<= 1;
            }
        }
    }

    pub(super) fn reset_sprite_evaluation(&mut self) {
        self.sprite_count = 0;
        self.sprite_zero_in_range_next = false;

        // Reset sprite overflow flag at the start of each scanline's sprite evaluation
        self.status_register.set_sprite_overflow(false);
    }
}
