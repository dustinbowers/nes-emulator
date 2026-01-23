use crate::messenger::Messenger;
use crate::{EmulatorMessage, WasmEventSource};
use nes_app::app::App;

pub fn start() {
    spawn_eframe();
}

fn spawn_eframe() {
    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let canvas = get_canvas("nes_canvas");
        let runner = eframe::WebRunner::new();

        let event_source = WasmEventSource::new();
        let app = App::new(event_source).with_logger(|msg| {
            web_sys::console::log_1(&msg.into());
        });

        runner
            .start(canvas, web_options, Box::new(|_cc| Ok(Box::new(app))))
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
