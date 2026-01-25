#![cfg(target_arch = "wasm32")]
#![warn(clippy::all, rust_2018_idioms)]
use crate::messenger::Messenger;
use eframe::egui;
use nes_app::app::{App, AppEvent, AppEventSource};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use wasm_bindgen::prelude::*;

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
        let messenger: Messenger<EmulatorMessage> = Messenger::new();
        messenger.init_message_listener();
        Self { messenger }
    }
}

impl AppEventSource for WasmEventSource {
    fn poll_event(&mut self) -> Option<AppEvent> {
        self.messenger.receive().map(|cmd| match cmd {
            EmulatorMessage::LoadRom(rom) => AppEvent::RequestLoadRom(rom),
            EmulatorMessage::Reset => AppEvent::RequestReset,
            EmulatorMessage::Pause => AppEvent::RequestPause,
        })
    }
}

thread_local! {
    static APP: RefCell<Option<Rc<RefCell<App<WasmEventSource>>>>> = RefCell::new(None);
}

#[wasm_bindgen]
pub fn start_audio() -> Result<(), JsValue> {
    APP.with(|cell| {
        let app = cell.borrow();
        let app = app
            .as_ref()
            .ok_or_else(|| JsValue::from_str("App not initialized"))?;
        app.borrow_mut()
            .init_audio()
            .map_err(|e| JsValue::from_str(&e.to_string()))
    })
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    web_sys::console::log_1(&"start() called".into());
    spawn_eframe()
}

fn spawn_eframe() {
    // Prevent multiple initializations
    static INITIALIZED: AtomicBool = AtomicBool::new(false);

    if INITIALIZED.swap(true, Ordering::SeqCst) {
        web_sys::console::log_1(&"Already initialized, skipping".into());
        return;
    }

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let canvas = get_canvas("nes_canvas");
        let runner = eframe::WebRunner::new();

        runner
            .start(
                canvas,
                web_options,
                Box::new(|_cc| {
                    let event_source = WasmEventSource::new();

                    let app = Rc::new(RefCell::new(App::new(event_source).with_logger(|msg| {
                        web_sys::console::log_1(&msg.into());
                    })));

                    // save handle for js -> wasm calls
                    APP.with(|cell| {
                        *cell.borrow_mut() = Some(app.clone());
                    });

                    Ok(Box::new(AppWrapper(app)))
                }),
            )
            .await
            .expect("eframe start failed");
    });
}

fn get_canvas(id: &str) -> web_sys::HtmlCanvasElement {
    use wasm_bindgen::JsCast;

    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id(id)
        .unwrap()
        .dyn_into()
        .unwrap()
}

struct AppWrapper(Rc<RefCell<App<WasmEventSource>>>);

impl eframe::App for AppWrapper {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.0.borrow_mut().update(ctx, frame);
    }
}
