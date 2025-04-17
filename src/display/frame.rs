use super::color_map::COLOR_MAP;
use super::consts::{FRAME_COLS, FRAME_ROWS};
use macroquad::color::Color;


// TODO: Remove this whole struct
#[deprecated]
pub struct Frame {
    pub data: Vec<Color>,
}

impl Frame {
    pub fn new() -> Self {
        Frame {
            data: vec![Color::new(0., 0., 0., 0.); FRAME_COLS * FRAME_ROWS],
        }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        // let index = y * FRAME_COLS + x;
        // self.data[index] = color;
        if x >= FRAME_COLS || y >= FRAME_ROWS {
            // println!("Pixel out of bounds: x={}, y={}", x, y);
        } else {
            self.data[y * FRAME_COLS + x] = color;
        }
    }

    pub fn show_tile(&mut self, chr_rom: &Vec<u8>, bank: usize, tile_n: usize) {
        assert!(bank <= 1);
        let bank_offset = bank * 0x1000;
        let tile = &chr_rom[(bank_offset + tile_n * 16)..(bank_offset + tile_n * 16 + 16)];
        for y in 0..=7 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];

            for x in (0..=7).rev() {
                let value = (1 & upper) << 1 | (1 & lower);
                upper >>= 1;
                lower >>= 1;
                let color = match value {
                    0 => COLOR_MAP.get_color(0x0D),
                    1 => COLOR_MAP.get_color(0x17),
                    2 => COLOR_MAP.get_color(0x28),
                    3 => COLOR_MAP.get_color(0x22),
                    _ => panic!("Impossible color palette index"),
                };
                let tile_index = tile_n * 8 + x;

                let tile_x = tile_index % 232;
                let tile_y = (tile_index / 232) * 8 + y + (bank * (80));
                self.set_pixel(tile_x, tile_y, *color);
            }
        }
    }
}
