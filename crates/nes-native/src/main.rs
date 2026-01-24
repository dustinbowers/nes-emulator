#![feature(get_mut_unchecked)]
#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use eframe::NativeOptions;
use nes_app::app::{App, AppCommand, AppEvent, AppEventSource};

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

    let events = NativeEventSource::new();
    let mut initial_commands = vec![];

    // Check if ROM was provided via command line
    if let Some(rom_path) = std::env::args().nth(1) {
        match std::fs::read(&rom_path) {
            Ok(rom_data) => {
                initial_commands.push(AppCommand::LoadRom(rom_data));
            }
            Err(e) => {
                eprintln!("Failed to load ROM '{}': {}", rom_path, e);
            }
        }
    }

    eframe::run_native(
        "NES Emulator",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(App::new_with_autostart(
                events,
                true,
                initial_commands,
            )))
        }),
    )
}
