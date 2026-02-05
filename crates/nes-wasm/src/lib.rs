#![cfg(target_arch = "wasm32")]
#![warn(clippy::all, rust_2018_idioms)]
use crate::messenger::Messenger;
use eframe::egui;
use nes_app::app::app::App;
use nes_app::app::event::{AppEvent, AppEventSource};
use once_cell::unsync::OnceCell;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use wasm_bindgen::prelude::*;

mod messenger;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "rom")]
pub enum ClientMessage {
    // JS to WASM
    LoadRom(Vec<u8>),
    Reset,
    Pause,
}

pub struct WasmEventSource {
    messenger: Messenger<ClientMessage>,
}

impl WasmEventSource {
    pub fn new() -> Self {
        let messenger: Messenger<ClientMessage> = Messenger::new();
        messenger.init_message_listener();
        Self { messenger }
    }
}

impl AppEventSource for WasmEventSource {
    fn poll_event(&mut self) -> Option<AppEvent> {
        self.messenger.receive().map(|cmd| match cmd {
            ClientMessage::LoadRom(rom) => AppEvent::LoadRom(rom),
            ClientMessage::Reset => AppEvent::Reset,
            ClientMessage::Pause => AppEvent::Pause,
        })
    }
}

thread_local! {
    static APP: RefCell<OnceCell<Rc<RefCell<App<WasmEventSource>>>>> =
        RefCell::new(OnceCell::new());
}

fn set_app(app: Rc<RefCell<App<WasmEventSource>>>) {
    APP.with(|cell| {
        let _ = cell.borrow_mut().set(app);
    });
}

fn with_app<R>(f: impl FnOnce(&Rc<RefCell<App<WasmEventSource>>>) -> R) -> Result<R, JsValue> {
    APP.with(|cell| {
        let cell = cell.borrow();
        let app = cell
            .get()
            .ok_or_else(|| JsValue::from_str("App not initialized"))?;
        Ok(f(app))
    })
}

#[wasm_bindgen]
pub fn start_emulator() -> Result<(), JsValue> {
    with_app(|app| {
        app.borrow_mut().start_emulator(); // Must be called in a user-interaction context
    })?;
    Ok(())
}

static INITIALIZED: AtomicBool = AtomicBool::new(false);

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    if INITIALIZED.swap(true, Ordering::SeqCst) {
        web_sys::console::log_1(&"Already initialized, skipping".into());
        return;
    }
    spawn_eframe();
}

fn spawn_eframe() {
    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async move {
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

                    // Expose to JS calls (gesture-sensitive start_emulator)
                    set_app(app.clone());

                    Ok(Box::new(AppWrapper { app }))
                }),
            )
            .await
            .expect("eframe start failed");
    });
}

fn get_canvas(id: &str) -> web_sys::HtmlCanvasElement {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id(id)
        .unwrap()
        .dyn_into()
        .unwrap()
}

struct AppWrapper {
    app: Rc<RefCell<App<WasmEventSource>>>,
}

impl eframe::App for AppWrapper {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.app.borrow_mut().update(ctx, frame);
    }
}
