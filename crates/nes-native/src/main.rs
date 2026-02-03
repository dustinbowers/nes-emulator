#![feature(get_mut_unchecked)]
#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use nes_app::app::app::App;
use nes_app::app::event::{AppEvent, AppEventSource};

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
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([256.0 * 3.0, 240.0 * 3.0])
            .with_title("NES Emulator"),
        ..Default::default()
    };

    let mut initial_events = vec![AppEvent::Start];
    if let Some(rom_path) = std::env::args_os().nth(1) {
        match std::fs::read(&rom_path) {
            Ok(rom_data) => initial_events.push(AppEvent::LoadRom(rom_data)),
            Err(e) => eprintln!("Failed to load ROM '{}': {e}", rom_path.to_string_lossy()),
        }
    }

    let events = NativeEventSource::new();

    eframe::run_native(
        "NES Emulator",
        options,
        Box::new(move |_cc| {
            let app = App::new(events)
                .with_logger(|msg| println!("{msg}"))
                .with_initial_events(initial_events);
            Ok(Box::new(app))
        }),
    )
}
