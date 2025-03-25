use macroquad::color::Color;
use macroquad::color_u8;
use macroquad::prelude::{draw_rectangle};
use crate::consts::*;

pub type Screen = Vec<Vec<Vec<bool>>>;



pub fn draw_screen(screen: &Screen) {
    for (ri, r) in screen.iter().enumerate() {
        for (ci, c) in r.iter().enumerate() {
            let mut color_ind: usize = 0;
            for (i, c) in c.iter().enumerate() {
                if *c {
                    color_ind |= 1 << i;
                }
            }
            // let color = color_map[color_ind as usize];
            let color = color_u8!(255, 255, 255, 255);

            let x = ci as f32 * PIXEL_WIDTH;
            let y = ri as f32 * PIXEL_HEIGHT;
            draw_rectangle(x, y, PIXEL_WIDTH, PIXEL_HEIGHT, color);
        }
    }
}