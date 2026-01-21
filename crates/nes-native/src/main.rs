#![feature(get_mut_unchecked)]
#![warn(clippy::all, rust_2018_idioms)]
// #![allow(unused_imports, dead_code, unused_variables)] // TODO: Remove this later
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::sync::atomic::Ordering;
use macroquad::prelude::*;
use nes_app::app::App;
use nes_app::{ROM_DATA, TRIGGER_LOAD, TRIGGER_RESET};

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

pub fn set_rom_data(rom_bytes: Vec<u8>) {
    let mut rom_data = ROM_DATA.lock().unwrap();
    *rom_data = rom_bytes;
    TRIGGER_LOAD.store(true, Ordering::SeqCst);
    TRIGGER_RESET.store(false, Ordering::SeqCst);
}

#[macroquad::main(window_conf)]
async fn main() {
    // Native App
    #[cfg(not(target_arch = "wasm32"))]
    {
        let args: Vec<String> = std::env::args().collect();
        if args.len() < 2 {
            eprintln!("Usage: {} <iNES 1.0 ROM path>", args[0]);
        }
        let rom_data = std::fs::read(args[1].clone()).expect("File not found");
        let mut app = App::new();
        set_rom_data(rom_data);
        app.run().await;
    }

    // WASM App
    #[cfg(target_arch = "wasm32")]
    {
        let mut app = App::new();
        app.run().await;
    }
}
