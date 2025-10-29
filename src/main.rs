#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod nes;
mod display;
mod app;

use crate::app::EmulatorApp;

#[cfg(not(target_arch = "wasm32"))]
use {
    std::process,
    std::env
};


// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {

    let args: Vec<String> = env::args().collect();

    // Ensure correct number of arguments
    if args.len() != 2 {
        eprintln!("Usage: {} <iNES 1.0 ROM path>", args[0]);
        process::exit(1);
    }
    let rom_path = &args[1];
    let rom_data = std::fs::read(rom_path).expect("Error reading ROM file.");
    // let rom = match Rom::new(&rom_data) {
    //     Ok(rom) => rom,
    //     Err(rom_error) => {
    //         println!("Error parsing rom: {:}", rom_error);
    //         return Ok(());
    //     }
    // };
    
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    
    

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0])
        // .with_icon(
        //     // NOTE: Adding an icon is optional
        //     eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
        //         .expect("Failed to load icon"),
        // ),
        ,
        ..Default::default()
    };
    eframe::run_native(
        "NES",
        native_options,
        Box::new(|cc| {
            let mut app = Box::new(EmulatorApp::new(cc));
            app.load_rom_data(&rom_data);
            Ok(app)
        }),
    )
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(EmulatorApp::new(cc)))),
            )
            .await;

        // Remove the loading text and spinner:
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}
