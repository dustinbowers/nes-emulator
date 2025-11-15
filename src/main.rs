#![feature(get_mut_unchecked)]
#![warn(clippy::all, rust_2018_idioms)]
#![allow(unused_imports, dead_code, unused_variables)] // TODO: Remove this later
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod display;
mod error;
mod nes;

// #[cfg(not(target_arch = "wasm32"))]
// mod app_native;
// #[cfg(target_arch = "wasm32")]
// mod app_wasm;
mod app;

// #[cfg(target_arch = "wasm32")]
// use crate::app_wasm::AppWasm;
// #[cfg(not(target_arch = "wasm32"))]
// use {
//     crate::app_native::AppNative,
//     std::{env, fs, process},
// };

use crate::app::App;
use crate::app::set_rom_data;

use crate::display::consts::*;
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "NES".to_owned(),
        fullscreen: false,
        window_height: WINDOW_HEIGHT as i32,
        window_width: WINDOW_WIDTH as i32,
        ..Default::default()
    }
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
