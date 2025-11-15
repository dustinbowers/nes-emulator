use crate::display::consts::{WINDOW_HEIGHT, WINDOW_WIDTH};
use crate::error::{EmulatorError, EmulatorErrorType};
use crate::nes::NES;
use crate::nes::cartridge::rom::Rom;
use crate::nes::controller::joypad::JoypadButton;
use macroquad::prelude::*;
use std::cell::UnsafeCell;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use tinyaudio::prelude::*;
use wasm_bindgen::prelude::wasm_bindgen;

pub struct NesCell(UnsafeCell<NES>);

// Safety: only the audio thread will ever tick the NES while running
unsafe impl Send for NesCell {}
unsafe impl Sync for NesCell {}

impl NesCell {
    pub fn new(nes: NES) -> Arc<Self> {
        Arc::new(Self(UnsafeCell::new(nes)))
    }

    #[inline(always)]
    pub unsafe fn get_mut(&self) -> &mut NES {
        unsafe { &mut *self.0.get() }
    }

    #[inline(always)]
    pub unsafe fn get_ref(&self) -> &NES {
        unsafe { &*self.0.get() }
    }
}

struct SharedInput {
    buttons: AtomicU8,
}
static CONTROLLER1: SharedInput = SharedInput {
    buttons: AtomicU8::new(0),
};

pub static TRIGGER_LOAD: AtomicBool = AtomicBool::new(false);
pub static TRIGGER_RESET: AtomicBool = AtomicBool::new(false);
pub static ROM_DATA: Mutex<Vec<u8>> = Mutex::new(Vec::new());

#[wasm_bindgen]
pub fn set_rom_data(rom_bytes: js_sys::Uint8Array) {
    web_sys::console::log_1(&"set_rom_data".into());
    let mut rom_data = ROM_DATA.lock().unwrap();
    *rom_data = rom_bytes.to_vec();
}

#[wasm_bindgen]
pub fn trigger_load() {
    web_sys::console::log_1(&"trigger_load()".into());
    TRIGGER_LOAD.store(true, Ordering::SeqCst);
}

#[wasm_bindgen]
pub fn trigger_reset() {
    web_sys::console::log_1(&"trigger_reset()".into());
    TRIGGER_RESET.store(true, Ordering::SeqCst);
}

enum State {
    Start,
    Waiting,
    Running,
    Error,
}

pub struct AppWasm {
    nes_arc: Arc<NesCell>,
    key_map: HashMap<KeyCode, JoypadButton>,
    pixel_buffer: Vec<u8>,
    texture: Option<Texture2D>,
    audio_device: Option<tinyaudio::OutputDevice>,
    state: State,
}

impl AppWasm {
    pub fn new() -> Self {
        let mut nes = NES::new();
        nes.set_sample_frequency(44_100);
        let nes_arc = NesCell::new(nes);

        let key_map_data: &[(KeyCode, JoypadButton)] = &[
            (KeyCode::K, JoypadButton::BUTTON_A),
            (KeyCode::J, JoypadButton::BUTTON_B),
            (KeyCode::Enter, JoypadButton::START),
            (KeyCode::Space, JoypadButton::SELECT),
            (KeyCode::W, JoypadButton::UP),
            (KeyCode::S, JoypadButton::DOWN),
            (KeyCode::A, JoypadButton::LEFT),
            (KeyCode::D, JoypadButton::RIGHT),
        ];

        let mut key_map = HashMap::new();
        for (k, v) in key_map_data.iter() {
            key_map.insert(*k, *v);
        }

        Self {
            nes_arc,
            key_map,
            pixel_buffer: vec![0; 256 * 240 * 4],
            texture: None,
            audio_device: None,
            state: State::Start,
        }
    }

    pub fn init_audio(&mut self) -> Result<(), EmulatorError> {
        Self::log(&"init_audio()".to_owned());
        let nes_clone = self.nes_arc.clone();
        let audio_device = run_output_device(
            OutputDeviceParameters {
                channels_count: 1,
                sample_rate: 44_100,
                channel_sample_count: 1200,
            },
            move |data| {
                let nes = unsafe { nes_clone.get_mut() };
                nes.bus
                    .controller1
                    .set_buttons(CONTROLLER1.buttons.load(Ordering::Relaxed));

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
                    *sample = raw;
                }
                nes.cycle_acc = cycle_acc;
            },
        );

        match audio_device {
            Ok(audio_device) => {
                self.audio_device = Some(audio_device);
                Ok(())
            }
            _ => {
                self.audio_device = None;
                Err(EmulatorError::new(
                    EmulatorErrorType::AudioInitFailed,
                    "init_audio()".to_string(),
                ))
            }
        }
    }

    pub async fn run(&mut self) {
        loop {
            match self.state {
                State::Start => {
                    self.state = State::Waiting;
                }
                State::Waiting => {
                    Self::log("[rust] waiting...");
                    if TRIGGER_LOAD.swap(false, Ordering::Relaxed) {
                        if let Err(_) = self.init_audio() {
                            self.state = State::Error;
                            continue;
                        }
                        let nes = unsafe { self.nes_arc.get_mut() };
                        if let Ok(cart) = Rom::new(&ROM_DATA.lock().unwrap().clone()) {
                            nes.insert_cartridge(cart.into());
                            self.state = State::Running;
                            TRIGGER_RESET.store(false, Ordering::Relaxed);
                        } else {
                            self.state = State::Error;
                        }
                    }
                    let alpha = ((get_time() % 2.0) / 2.0) as f32;
                    let size = 48.0;
                    let str = "Insert a Cartridge";
                    let x = WINDOW_WIDTH as f32 / 2.0 - (size / 2.0 * str.len() as f32 / 2.0);
                    let y = WINDOW_HEIGHT as f32 / 2.0;

                    clear_background(Color::new(0.1, 0.1, 0.1, 1.0));
                    draw_text(str, x, y, size, Color::new(1.0, 1.0, 1.0, alpha));
                }
                State::Running => {
                    Self::log("[rust] running...");
                    self.handle_input();
                    self.render();

                    if TRIGGER_RESET.swap(false, Ordering::Relaxed) {
                        self.reset();
                        self.state = State::Waiting;
                    }
                }
                State::Error => {
                    panic!("error"); // TODO: handle gracefullt
                }
            }
            next_frame().await;
        }
    }

    fn handle_input(&mut self) {
        let keys_down = get_keys_down();
        for (key, button) in self.key_map.iter() {
            if keys_down.contains(key) {
                CONTROLLER1
                    .buttons
                    .fetch_or(button.bits(), Ordering::Relaxed);
            } else {
                CONTROLLER1
                    .buttons
                    .fetch_and(!button.bits(), Ordering::Relaxed);
            }
        }
    }

    fn reset(&mut self) {
        let nes = unsafe { self.nes_arc.get_mut() };
        nes.bus.reset();
    }

    pub fn render(&mut self) {
        clear_background(BLACK);

        let nes = unsafe { self.nes_arc.get_ref() };
        let frame = nes.get_frame_buffer();

        for (i, &p) in frame.iter().enumerate() {
            let color = crate::display::color_map::SYSTEM_PALETTE[p as usize];
            let base = i * 4;
            self.pixel_buffer[base + 0] = color.0;
            self.pixel_buffer[base + 1] = color.1;
            self.pixel_buffer[base + 2] = color.2;
            self.pixel_buffer[base + 3] = 255;
        }

        if self.texture.is_none() {
            self.texture = Some(Texture2D::from_rgba8(256, 240, &self.pixel_buffer));
            self.texture
                .as_ref()
                .unwrap()
                .set_filter(FilterMode::Linear);
        } else {
            self.texture
                .as_ref()
                .unwrap()
                .update_from_bytes(256, 240, &self.pixel_buffer);
        }

        let tex = self.texture.as_ref().unwrap();
        draw_texture_ex(
            tex,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(screen_width(), screen_height())),
                ..Default::default()
            },
        );
    }

    fn log(msg: &str) {
        #[cfg(feature = "logging")]
        web_sys::console::log_1(&msg.into());
    }
}
