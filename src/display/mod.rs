pub mod color_map;
pub mod consts;
pub mod frame;
pub mod render;

use consts::{PIXEL_HEIGHT, PIXEL_WIDTH};
use frame::Frame;

use consts::FRAME_COLS;
use macroquad::prelude::draw_rectangle;

pub fn draw_frame(frame: &Frame) {
    for (ind, c) in frame.data.iter().enumerate() {
        let x = (ind % FRAME_COLS) as f32 * PIXEL_WIDTH;
        let y = (ind / FRAME_COLS) as f32 * PIXEL_HEIGHT;
        draw_rectangle(x, y, PIXEL_WIDTH, PIXEL_HEIGHT, *c);
    }
}
