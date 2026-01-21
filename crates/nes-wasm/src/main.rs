#![warn(clippy::all, rust_2018_idioms)]
use std::sync::atomic::Ordering;
use macroquad::prelude::*;
use wasm_bindgen::prelude::*;
use nes_app::{ROM_DATA, TRIGGER_LOAD, TRIGGER_RESET};
use nes_app::app::App;

const WINDOW_HEIGHT: u32 = 480;
const WINDOW_WIDTH: u32 = 512;

fn window_conf() -> Conf {
    Conf {
        window_title: "NES".to_owned(),
        fullscreen: false,
        window_height: WINDOW_HEIGHT as i32,
        window_width: WINDOW_WIDTH as i32,
        ..Default::default()
    }
}

#[wasm_bindgen]
pub fn set_rom_data(rom_bytes: js_sys::Uint8Array) {
    trigger_reset();
    let mut rom_data = ROM_DATA.lock().unwrap();
    *rom_data = rom_bytes.to_vec();
    web_sys::console::log_1(&format!("set_rom_data() with {} bytes", (*rom_data).len()).into());
    trigger_load();
}

#[wasm_bindgen]
pub fn trigger_load() {
    web_sys::console::log_1(&"trigger_load()".into());
    // TRIGGER_RESET.store(false, Ordering::SeqCst);
    TRIGGER_LOAD.store(true, Ordering::SeqCst);
}

#[wasm_bindgen]
pub fn trigger_reset() {
    web_sys::console::log_1(&"trigger_reset()".into());
    // TRIGGER_LOAD.store(false, Ordering::SeqCst);
    TRIGGER_RESET.store(true, Ordering::SeqCst);
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut app = App::new();
    app.run().await;
}
