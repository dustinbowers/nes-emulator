pub mod color_map;
pub mod frame;
pub mod consts;
pub mod render;

use color_map::ColorMap;
use consts::{PIXEL_HEIGHT, PIXEL_WIDTH};
use frame::Frame;

use macroquad::prelude::draw_rectangle;
use crate::display::consts::{FRAME_COLS, FRAME_ROWS};

pub type Screen = Vec<Vec<u8>>;

// pub fn draw_screen(screen: &Screen, color_map: &ColorMap) {
//     for (ri, r) in screen.iter().enumerate() {
//         for (ci, c) in r.iter().enumerate() {
//             let color = color_map.get_color(*c as usize);
//             // let color = color_u8!(255, 255, 255, 255);
//
//             let x = ci as f32 * PIXEL_WIDTH;
//             let y = ri as f32 * PIXEL_HEIGHT;
//             draw_rectangle(x, y, PIXEL_WIDTH, PIXEL_HEIGHT, *color);
//         }
//     }
// }

pub fn draw_frame(frame: &Frame) {
    for (ind, c) in frame.data.iter().enumerate() {
        let x = (ind % FRAME_COLS) as f32 * PIXEL_WIDTH;
        let y = (ind / FRAME_COLS) as f32 * PIXEL_HEIGHT;
        draw_rectangle(x, y, PIXEL_WIDTH, PIXEL_HEIGHT, *c);
    }
}
