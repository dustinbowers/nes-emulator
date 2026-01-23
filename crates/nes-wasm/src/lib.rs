#![warn(clippy::all, rust_2018_idioms)]
use crate::messenger::Messenger;
use nes_app::app::{AppEvent, AppEventSource};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

mod bootstrap;
mod messenger;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "rom")]
pub enum EmulatorMessage {
    // JS to WASM
    LoadRom(Vec<u8>),
    Reset,
    Pause,
}

pub struct WasmEventSource {
    messenger: Messenger<EmulatorMessage>,
}

impl Default for WasmEventSource {
    fn default() -> Self {
        Self::new()
    }
}

impl WasmEventSource {
    pub fn new() -> Self {
        let messenger = Messenger::new();
        messenger.init_message_listener();
        Self { messenger }
    }
}

impl AppEventSource for WasmEventSource {
    fn poll_event(&mut self) -> Option<AppEvent> {
        self.messenger.receive().map(|cmd| match cmd {
            EmulatorMessage::LoadRom(rom) => AppEvent::LoadRom(rom),
            EmulatorMessage::Reset => AppEvent::Reset,
            EmulatorMessage::Pause => AppEvent::Pause,
        })
    }
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    bootstrap::start();
}
