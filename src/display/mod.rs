pub mod color_map;

use macroquad::color::Color;
use macroquad::color_u8;
use macroquad::prelude::{draw_rectangle};
use crate::consts::*;
use crate::display::color_map::ColorMap;

pub type Screen = Vec<Vec<u8>>;



pub fn draw_screen(screen: &Screen, color_map: &ColorMap) {
    for (ri, r) in screen.iter().enumerate() {
        for (ci, c) in r.iter().enumerate() {
            let color = color_map.get_color(*c as usize);
            // let color = color_u8!(255, 255, 255, 255);

            let x = ci as f32 * PIXEL_WIDTH;
            let y = ri as f32 * PIXEL_HEIGHT;
            draw_rectangle(x, y, PIXEL_WIDTH, PIXEL_HEIGHT, *color);
        }
    }
}