#![feature(get_mut_unchecked)]
#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::NativeOptions;
use nes_app::app::{App, AppEvent, AppEventSource};
use nes_app::{ROM_DATA, TRIGGER_LOAD};
use std::sync::atomic::Ordering;
use std::sync::mpsc::Receiver;

pub struct NativeEventSource {
    // rx: Receiver<AppEvent>,
}

impl NativeEventSource {
    pub fn new() -> Self {
        Self {}
    }
}
impl AppEventSource for NativeEventSource {
    fn poll_event(&mut self) -> Option<AppEvent> {
        // self.rx.try_recv().ok()
        None
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([256.0 * 3.0, 240.0 * 3.0])
            .with_title("NES Emulator"),
        ..Default::default()
    };

    // Check if ROM was provided via command line
    if let Some(rom_path) = std::env::args().nth(1) {
        match std::fs::read(&rom_path) {
            Ok(rom_data) => {
                *ROM_DATA.lock().unwrap() = rom_data;
                TRIGGER_LOAD.store(true, Ordering::SeqCst);
            }
            Err(e) => {
                eprintln!("Failed to load ROM '{}': {}", rom_path, e);
            }
        }
    }

    let events = NativeEventSource::new();
    eframe::run_native(
        "NES Emulator",
        options,
        Box::new(move |_cc| Ok(Box::new(App::new_with_autostart(events, true)))),
    )
}
