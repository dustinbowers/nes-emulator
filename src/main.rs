#![feature(get_mut_unchecked)]
#![warn(clippy::all, rust_2018_idioms)]
#![allow(unused_imports, dead_code, unused_variables)] // TODO: Remove this later
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod app;
mod display;
mod error;
mod nes;

use crate::app::EmulatorApp;
use crate::display::consts::*;
use macroquad::prelude::*;

#[cfg(target_arch = "wasm32")]
use {
    wasm_bindgen::JsCast,
    wasm_bindgen_futures::JsFuture,
    web_sys::{Request, RequestInit, RequestMode, Response},
};

#[cfg(not(target_arch = "wasm32"))]
use {std::env, std::process};

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
#[macroquad::main(window_conf)]
async fn main() {
    // Decide ROM path relative to your served page. Common setups use "roms/SMB.nes".
    // You can also provide a UI file-picker instead of a hard-coded path.
    let rom_path = "roms/SMB.nes";

    // fetch the rom bytes (returns Vec<u8>)
    match fetch_bytes(rom_path).await {
        Ok(rom_data) => {
            let mut app = EmulatorApp::new();
            app.load_rom_data(&rom_data);
            app.run().await;
        }
        Err(err) => {
            // Show an error message on the page (console.log)
            web_sys::console::error_1(
                &format!("Failed to load ROM {}: {:?}", rom_path, err).into(),
            );
            // Optionally: enter a "no-rom" UI or block
            // But we'll still start the app so the page doesn't do nothing
            let mut app = EmulatorApp::new();
            app.run().await;
        }
    }
}

#[cfg(target_arch = "wasm32")]
async fn fetch_bytes(path: &str) -> anyhow::Result<Vec<u8>> {
    // Use web_sys fetch to retrieve the ROM as an ArrayBuffer, then copy to Vec<u8>.
    // Returns Err if fetch or conversion fails.
    let window = web_sys::window().ok_or_else(|| anyhow::anyhow!("No window"))?;
    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(path, &opts)
        .map_err(|e| anyhow::anyhow!("Request creation failed: {:?}", e))?;
    // Optionally set headers: request.headers().set("Accept", "application/octet-stream")?;

    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| anyhow::anyhow!("Fetch failed: {:?}", e))?;
    // let resp: Response = resp_value.map_err(|_| anyhow::anyhow!("Failed to cast Response"))?;
    let resp: Response = resp_value
        .dyn_into()
        .map_err(|_| anyhow::anyhow!("Failed to cast Response"))?;
    if !resp.ok() {
        return Err(anyhow::anyhow!("HTTP error: {}", resp.status()));
    }

    let ab_promise = resp
        .array_buffer()
        .map_err(|e| anyhow::anyhow!("array_buffer() failed: {:?}", e))?;
    let ab = JsFuture::from(ab_promise)
        .await
        .map_err(|e| anyhow::anyhow!("array_buffer promise failed: {:?}", e))?;
    let u8_array = js_sys::Uint8Array::new(&ab);
    let mut v = vec![0u8; u8_array.length() as usize];
    u8_array.copy_to(&mut v[..]);
    Ok(v)
}
