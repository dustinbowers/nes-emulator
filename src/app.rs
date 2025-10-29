// use rand::random;

use crate::display::*;
use crate::display::color_map::COLOR_MAP;
use crate::display::consts::WINDOW_WIDTH;
use crate::nes::cartridge::rom::Rom;
use crate::nes::NES;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
// #[derive(serde::Deserialize, serde::Serialize)]
// #[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct EmulatorApp {
    value: f32,
    
    nes: NES,
}

impl Default for EmulatorApp {
    fn default() -> Self {
        Self {
            value: 2.7,
            nes: NES::new(),
        }
    }
}

impl EmulatorApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Default::default()
    }
    
    pub fn load_rom_data(&mut self, rom_bytes: &Vec<u8>) {
        match NES::parse_rom_bytes(rom_bytes) {
            Ok(cart) => {
                println!("Loading {} rom bytes...", rom_bytes.len());
                self.nes.insert_cartridge(cart);
            },
            Error => panic!("Bad ROM data!") // TODO: handle this gracefully
        }
    }
}

impl eframe::App for EmulatorApp {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
     
        // TODO: remove this later, it's just for testing
        for i in 0..29780 {
            self.nes.tick();
        }
        
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Load ROM...").clicked() {
                            let rom_path = "roms/SMB.nes";
                            let rom_data = std::fs::read(rom_path).expect("Error reading ROM file.");
                            let rom = match Rom::new(&rom_data) {
                                Ok(rom) => rom,
                                Err(rom_error) => {
                                    panic!("Error parsing rom: {:#?}", rom_error);
                                }
                            };

                        }
                        if ui.button("Exit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_theme_preference_buttons(ui);
            });
        });

        let frame = self.nes.get_frame_buffer();

        let width = 256;
        let height = 240;
        let mut pixel_buffer: Vec<egui::Color32> = Vec::with_capacity(width * height);

        for (i, c) in frame.iter().enumerate() {
            let x = i % 256;
            let y = i / 256;

            let color = COLOR_MAP.get_color((*c) as usize);
            let mut ind = (y as usize) * (WINDOW_WIDTH as usize) + x as usize;
            pixel_buffer.push(*color);
        }
       
        let color_image = egui::ColorImage {
            size: [width, height],
            source_size: Default::default(),
            pixels: pixel_buffer
        };

        let texture = ctx.load_texture(
            "my_pixel_buffer_texture",
            color_image,
            egui::TextureOptions::LINEAR, // Or egui::TextureOptions::NEAREST for pixel art
        );

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.image(&texture);
            ui.label(format!("PC: {:04x}", self.nes.bus.cpu.program_counter));
        });

        ctx.request_repaint();
    }
}
