use std::time::Duration;
use crate::display::*;
use crate::display::color_map::COLOR_MAP;
use crate::display::consts::WINDOW_WIDTH;
use crate::nes::cartridge::rom::Rom;
use crate::nes::NES;
use tinyaudio::prelude::*;

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::sync::Arc;
use eframe::epaint::textures::TextureOptions;
use egui::{Key, KeyboardShortcut};
use crate::nes::controller::joypad::JoypadButtons;

pub struct NesCell(UnsafeCell<NES>);

// Safety: only the audio thread will ever mutate the NES.
unsafe impl Send for NesCell {}
unsafe impl Sync for NesCell {}

impl NesCell {
    pub fn new(nes: NES) -> Arc<Self> {
        Arc::new(Self(UnsafeCell::new(nes)))
    }

    #[inline(always)]
    pub unsafe fn get_mut(&self) -> &mut NES {
        &mut *self.0.get()
    }

    #[inline(always)]
    pub unsafe fn get_ref(&self) -> &NES {
        &*self.0.get()
    }
}

struct SharedInput {
    buttons: AtomicU8, // bitmask of controller1 buttons
}
static CONTROLLER1: SharedInput = SharedInput {
    buttons: AtomicU8::new(0),
};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
// #[derive(serde::Deserialize, serde::Serialize)]
// #[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct EmulatorApp {
    nes_arc: Arc<NesCell>,
    audio_device: OutputDevice,
    keycode_to_joypad: HashMap<Key, JoypadButtons>,

    // Preallocated pixel buffer to avoid allocations every frame
    pixel_buffer: Vec<egui::Color32>,
    texture: Option<egui::TextureHandle>,
}

impl EmulatorApp {
    /// Called once before the first frame.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut nes = NES::new();
        nes.set_sample_frequency(44_100);
        let nes_arc = NesCell::new(nes);
        let nes_audio = nes_arc.clone();

        let key_map_data: &[(Vec<Key>, JoypadButtons)] = &[
            (vec![Key::K], JoypadButtons::BUTTON_A),
            (vec![Key::J], JoypadButtons::BUTTON_B),
            (vec![Key::Enter], JoypadButtons::START),
            (vec![Key::Space], JoypadButtons::SELECT),
            (vec![Key::W], JoypadButtons::UP),
            (vec![Key::S], JoypadButtons::DOWN),
            (vec![Key::A], JoypadButtons::LEFT),
            (vec![Key::D], JoypadButtons::RIGHT),
        ];

        let mut keycode_to_joypad: HashMap<Key, JoypadButtons> = HashMap::new();
        for (keycodes, button) in key_map_data.iter() {
            for keycode in keycodes.iter() {
                keycode_to_joypad.insert(*keycode, *button);
            }
        }

        let audio_device = run_output_device(
            OutputDeviceParameters {
                channels_count: 1,
                sample_rate: 44_100,
                channel_sample_count: 4410,
            },
            move |data| {
                unsafe {
                    let nes = nes_audio.get_mut();
                    nes.bus
                        .controller1
                        .set_buttons(CONTROLLER1.buttons.load(Ordering::Relaxed));
                    
                    for sample in data.iter_mut() {
                        nes.tick();
                        *sample = nes.bus.apu.sample();
                    }

                    // PPU cycles per audio sample (5.369318 MHz / 44.1 kHz)
                    let ppu_cycles_per_sample = 5369318.0 / 44100.0; // ~121.7 PPU cycles per sample
                    let mut cycle_acc = nes.cycle_acc;

                    for sample in data {
                        cycle_acc += ppu_cycles_per_sample;

                        while cycle_acc >= 1.0 {
                            nes.tick(); // tick at PPU frequency
                            cycle_acc -= 1.0;
                        }

                        let raw = nes.bus.apu.sample();
                        // let scaled = (raw * 32767.0) as i16;
                        // *sample = scaled as f32;
                        *sample = raw;
                    }

                    nes.cycle_acc = cycle_acc;
                }
            },
        ).expect("Failed to start audio output device");

        let pixel_buffer = vec![egui::Color32::BLACK; 256 * 240];

        Self {
            nes_arc,
            audio_device,
            keycode_to_joypad,
            pixel_buffer,
            texture: None,
        }
    }
    
    pub fn load_rom_data(&mut self, rom_bytes: &Vec<u8>) {
        match NES::parse_rom_bytes(rom_bytes) {
            Ok(cart) => {
                println!("Loading {} rom bytes...", rom_bytes.len());
                unsafe {
                    let nes: &mut NES = self.nes_arc.get_mut();
                    nes.insert_cartridge(cart);
                }
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

        ctx.input(|i| {
            for (key, joy_button) in self.keycode_to_joypad.iter() {
                if i.key_down(*key) {
                    CONTROLLER1.buttons.fetch_or((*joy_button).bits(), Ordering::Relaxed);
                } else {
                    CONTROLLER1.buttons.fetch_and(!(*joy_button).bits(), Ordering::Relaxed);
                }
            }
        });

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

        let nes = unsafe { self.nes_arc.get_ref() };
        let frame = nes.get_frame_buffer();

        // Overwrite the preallocated buffer
        for (i, c) in frame.iter().enumerate() {
            self.pixel_buffer[i] = *COLOR_MAP.get_color(*c as usize);
        }

        // Wrap buffer in ColorImage only once per frame without cloning
        let color_image = egui::ColorImage {
            size: [256, 240],
            pixels: std::mem::take(&mut self.pixel_buffer), // move out buffer temporarily
            source_size: Default::default(),
        };

        if let Some(tex) = self.texture.as_mut() {
            tex.set(color_image, TextureOptions::NEAREST);
        } else {
            self.texture = Some(ctx.load_texture(
                "nes_texture",
                color_image,
                TextureOptions::NEAREST,
            ));
        }

        // Put pixel_buffer back in place (so we can reuse it next frame)
        self.pixel_buffer = vec![egui::Color32::BLACK; 256 * 240];
      
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(tex) = &self.texture {
                ui.image(tex);
            }
            ui.label(format!("PC: {:04x}", nes.bus.cpu.program_counter));
        });

        // ctx.request_repaint_after(Duration::from_millis(16));
        ctx.request_repaint();
    }
}
