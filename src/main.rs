#![feature(get_mut_unchecked)]
#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod nes;
mod display;
mod app;
mod error;

use macroquad::prelude::*;
use crate::app::EmulatorApp;

#[cfg(not(target_arch = "wasm32"))]
use {
    std::process,
    std::env
};
use crate::display::color_map::COLOR_MAP;
use crate::display::consts::*;
use crate::nes::cartridge::rom::Rom;
use crate::nes::controller::joypad::JoypadButton;
use crate::nes::NES;

fn window_conf() -> Conf {
    Conf {
        window_title: "NES".to_owned(),
        fullscreen: false,
        window_height: WINDOW_HEIGHT as i32,
        window_width: WINDOW_WIDTH as i32,
        ..Default::default()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[macroquad::main(window_conf)]
async fn main() {
    let args: Vec<String> = env::args().collect();

    // Ensure correct number of arguments
    if args.len() != 2 {
        eprintln!("Usage: {} <iNES 1.0 ROM path>", args[0]);
        process::exit(1);
    }
    let rom_path = &args[1];
    let rom_data = std::fs::read(rom_path).expect("Error reading ROM file.");

    let mut app = EmulatorApp::new();
    app.load_rom_data(&rom_data);
    app.run().await;
}

#[cfg(target_arch = "wasm32")]
async fn main() {
    // TODO implement wasm 
}

