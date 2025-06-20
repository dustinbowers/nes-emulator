use crate::ppu::{PaletteKind, PPU};

impl PPU {
    /// `get_sprite_pixel` determines sprite pixel. Returns (sprite_color, sprite_in_front, sprite_zero_rendered)
    pub(super) fn get_sprite_pixel(&mut self) -> (u8, bool, bool) {
        let mut sprite_color = 0;
        let mut sprite_in_front = false;
        let mut sprite_zero_rendered = false;

        for i in 0..self.sprite_count {
            if self.sprite_x_counter[i] == 0 {
                let low_bit = (self.sprite_pattern_low[i] >> 7) & 1;
                let high_bit = (self.sprite_pattern_high[i] >> 7) & 1;
                let pixel = (high_bit << 1) | low_bit;

                if pixel != 0 {
                    let palette = self.sprite_attributes[i] & 0b11;
                    sprite_color = self.read_palette_color(palette, pixel, PaletteKind::Sprite);
                    sprite_in_front = (self.sprite_attributes[i] & 0b0010_0000) == 0;
                    if i == 0 {
                        sprite_zero_rendered = true;
                    }
                    break;
                }
            }
        }
        (sprite_color, sprite_in_front, sprite_zero_rendered)
    }

    pub(super) fn sprite_evaluation(&mut self, scanline: usize, dot: usize) {
        let current_sprite_index = (dot - 65) / 2;
        debug_assert!(current_sprite_index < 64);
        debug_assert!(dot % 2 == 0); // Only runs on even cycles

        let oam_base = 4 * current_sprite_index;
        let secondary_oam_base = self.sprite_count * 4; // sprite_count ranges 0..8

        let current_oam_y = self.oam_data[oam_base + 0] as usize;
        let sprite_height = self.ctrl_register.sprite_size() as i16;
        if (scanline as i16 - current_oam_y as i16) >= 0
            && (scanline as i16 - current_oam_y as i16) < sprite_height
        {
            if self.sprite_count < 8 {
                self.secondary_oam[secondary_oam_base + 0] = self.oam_data[oam_base + 0];
                self.secondary_oam[secondary_oam_base + 1] = self.oam_data[oam_base + 1];
                self.secondary_oam[secondary_oam_base + 2] = self.oam_data[oam_base + 2];
                self.secondary_oam[secondary_oam_base + 3] = self.oam_data[oam_base + 3];

                // Check if sprite0 is in range
                if self.sprite_count == 0 {
                    // TODO: The actual sprite_zero_hit isn't triggered until render time.
                    // self.status_register.set_sprite_zero_hit(true);

                    self.sprite_zero_in_range = true; // Look out for possible sprite0 hit
                }

                self.sprite_count += 1;
            } else {
                // Too many sprites!
                self.status_register.set_sprite_overflow(true);
            }
        }
    }

    pub(super) fn sprite_fill_register(&mut self, sprite_num: usize, scanline: usize) {
        if sprite_num < self.sprite_count {
            let base = 4 * sprite_num;

            let y = self.secondary_oam[base + 0];
            let tile_index = self.secondary_oam[base + 1];
            let attributes = self.secondary_oam[base + 2];
            let x = self.secondary_oam[base + 3];

            let sprite_height = self.ctrl_register.sprite_size() as i16;
            let mut row = (scanline as i16) - (y as i16);

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

            let mut pattern_low = self.read_bus(pattern_addr);
            let mut pattern_high = self.read_bus(pattern_addr + 8);

            // Horizontal flip
            let horizontal_flip = (attributes & 0x40) != 0;
            if horizontal_flip {
                pattern_low = pattern_low.reverse_bits();
                pattern_high = pattern_high.reverse_bits();
            }

            self.sprite_x_counter[sprite_num] = x;
            self.sprite_attributes[sprite_num] = attributes;
            self.sprite_pattern_low[sprite_num] = pattern_low;
            self.sprite_pattern_high[sprite_num] = pattern_high;
        } else if sprite_num == self.sprite_count {
            // TODO: First unused slot inherits Y from sprite #63 (quirk).
        }
    }

    pub(super) fn shift_sprite_registers(&mut self) {
        // Now shift sprite pattern registers for next pixel
        for i in 0..self.sprite_count {
            if self.sprite_x_counter[i] > 0 {
                self.sprite_x_counter[i] -= 1;
            } else {
                self.sprite_pattern_low[i] <<= 1;
                self.sprite_pattern_high[i] <<= 1;
            }
        }
    }
}
