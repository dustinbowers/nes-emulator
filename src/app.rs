use crate::display::consts::{WINDOW_HEIGHT, WINDOW_WIDTH};
use crate::error::{EmulatorError, EmulatorErrorType};
use crate::nes::NES;
use crate::nes::cartridge::rom::Rom;
use crate::nes::controller::joypad::JoypadButton;
use macroquad::prelude::*;
use std::cell::UnsafeCell;
use std::collections::{HashMap, VecDeque};
use std::error::Error;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use tinyaudio::prelude::*;

#[cfg(target_arch = "wasm32")]
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

// TODO: Potentially rename these to better reflect the "done-ness" of the respective states
pub static TRIGGER_LOAD: AtomicBool = AtomicBool::new(false);
pub static TRIGGER_RESET: AtomicBool = AtomicBool::new(false);
pub static PAUSE_EMULATION: AtomicBool = AtomicBool::new(true);
pub static ROM_DATA: Mutex<Vec<u8>> = Mutex::new(Vec::new());

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn set_rom_data(rom_bytes: js_sys::Uint8Array) {
    trigger_reset();
    let mut rom_data = ROM_DATA.lock().unwrap();
    *rom_data = rom_bytes.to_vec();
    web_sys::console::log_1(&format!("set_rom_data() with {} bytes", (*rom_data).len()).into());
    trigger_load();
}
#[cfg(not(target_arch = "wasm32"))]
pub fn set_rom_data(rom_bytes: Vec<u8>) {
    let mut rom_data = ROM_DATA.lock().unwrap();
    *rom_data = rom_bytes;
    TRIGGER_LOAD.store(true, Ordering::SeqCst);
    TRIGGER_RESET.store(false, Ordering::SeqCst);
}


#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn trigger_load() {
    web_sys::console::log_1(&"trigger_load()".into());
    // TRIGGER_RESET.store(false, Ordering::SeqCst);
    TRIGGER_LOAD.store(true, Ordering::SeqCst);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn trigger_reset() {
    web_sys::console::log_1(&"trigger_reset()".into());
    // TRIGGER_LOAD.store(false, Ordering::SeqCst);
    TRIGGER_RESET.store(true, Ordering::SeqCst);
}

enum State {
    Start,
    Waiting,
    Running,
    Error,
}

pub struct App {
    nes_arc: Arc<NesCell>,
    key_map: HashMap<KeyCode, JoypadButton>,
    pixel_buffer: Vec<u8>,
    texture: Option<Texture2D>,
    audio_device: Option<tinyaudio::OutputDevice>,
    state: State,
    error: Option<String>,
}

impl App {
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
            error: None,
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

                // if paused, send silence
                if PAUSE_EMULATION.load(Ordering::SeqCst) {
                    for s in data.iter_mut() { *s = 0.0; }
                    return;
                }
                
                nes.bus
                    .controller1
                    .set_buttons(CONTROLLER1.buttons.load(Ordering::SeqCst));

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
                   
                    let alpha = ((get_time() % 2.0) / 2.0) as f32;
                    let size = 48.0;
                    let str = "Insert a Cartridge";
                    let x = WINDOW_WIDTH as f32 / 2.0 - (size / 2.0 * str.len() as f32 / 2.0);
                    let y = WINDOW_HEIGHT as f32 / 2.0;

                    clear_background(Color::new(0.1, 0.1, 0.1, 1.0));
                    draw_text(str, x, y, size, Color::new(1.0, 1.0, 1.0, alpha));
                    if TRIGGER_LOAD.swap(false, Ordering::SeqCst) {
                        if self.audio_device.is_none() {
                            if let Err(_) = self.init_audio() {
                                self.set_error("Audio initialization failed!".to_owned())
                            }
                        }
                        let nes = unsafe { self.nes_arc.get_mut() };
                        let rom_data = ROM_DATA.lock().unwrap();
                        match Rom::new(&rom_data) {
                            Ok(rom) => {
                                match rom.into_cartridge() {
                                    Ok(cart) => {
                                        nes.insert_cartridge(cart);
                                        self.state = State::Running;
                                        TRIGGER_RESET.store(false, Ordering::SeqCst);
                                        PAUSE_EMULATION.store(false, Ordering::SeqCst);
                                    }
                                    Err(err) => {
                                        self.set_error(err.to_string());
                                    }
                                }
                            }
                            Err(err) => {
                                self.set_error(err.to_string());
                            }
                        }
                    }
                }
                State::Running => {
                    Self::log("[rust] running...");
                    self.handle_input();
                    self.render();

                    if TRIGGER_RESET.swap(false, Ordering::SeqCst) {
                        self.reset();
                        self.state = State::Waiting;
                        continue;
                    }
                    
                    // Error checking
                    let nes: &NES = unsafe { self.nes_arc.get_ref() };
                    if let Some(err) = &nes.bus.cpu.error {
                        self.set_error(err.to_string());
                        continue;
                    }
                }
                State::Error => {
                    let msg = self.error
                        .as_ref()
                        .map(|e| e.to_string())
                        .unwrap_or_else(|| "Unknown emulator error".to_string());

                    self.draw_error_screen(&msg);

                    // wait for user to press R
                    if is_key_pressed(KeyCode::R) 
                        || TRIGGER_RESET.swap(false, Ordering::SeqCst) {
                        self.reset();
                    }
                }
            }
            
            let nes: &mut NES = unsafe { self.nes_arc.get_mut() };
            if is_key_pressed(KeyCode::Key1) {
                nes.bus.apu.mute_pulse1 = !nes.bus.apu.mute_pulse1;
            }
            if is_key_pressed(KeyCode::Key2) {
                nes.bus.apu.mute_pulse2 = !nes.bus.apu.mute_pulse2;
            }
            if is_key_pressed(KeyCode::Key3) {
                nes.bus.apu.mute_triangle = !nes.bus.apu.mute_triangle;
            }
            if is_key_pressed(KeyCode::Key4) {
                nes.bus.apu.mute_noise = !nes.bus.apu.mute_noise;
            }
            if is_key_pressed(KeyCode::Key5) {
                nes.bus.apu.mute_dmc = !nes.bus.apu.mute_dmc;
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
                    .fetch_or(button.bits(), Ordering::SeqCst);
            } else {
                CONTROLLER1
                    .buttons
                    .fetch_and(!button.bits(), Ordering::SeqCst);
            }
        }
    }

    fn reset(&mut self) {
        PAUSE_EMULATION.store(true, Ordering::SeqCst);
        let mut rom_data = ROM_DATA.lock().unwrap();
        *rom_data = vec![];
        TRIGGER_LOAD.store(false, Ordering::SeqCst);
        TRIGGER_RESET.store(false, Ordering::SeqCst);
        self.error = None;
        self.state = State::Waiting;
        
        let nes: &mut NES = unsafe { self.nes_arc.get_mut() };
        nes.bus.reset();
    }

    pub fn render(&mut self) {
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
    
    fn set_error(&mut self, err: String) {
        PAUSE_EMULATION.store(true, Ordering::SeqCst);
        TRIGGER_RESET.store(false, Ordering::SeqCst);
        TRIGGER_LOAD.store(false, Ordering::SeqCst);
        self.error = Some(err.to_string());
        self.state = State::Error;
    }

    fn log(msg: &str) {
        #[cfg(feature = "logging")]
        {
            #[cfg(target_arch = "wasm32")]
            web_sys::console::log_1(&msg.into());
            #[cfg(not(target_arch = "wasm32"))]
            println!("{}", msg);
        }
    }
    pub fn draw_error_screen(&self, msg: &str) {
        let (w, h) = (screen_width(), screen_height());

        // Background
        draw_rectangle(0.0, 0.0, w, h, Color::new(0.05, 0.0, 0.0, 0.85));

        let title = "EMULATOR ERROR";
        let title_dim = measure_text(title, None, 40, 1.0);
        draw_text(
            title,
            w * 0.5 - title_dim.width * 0.5,
            h * 0.3,
            40.0,
            RED,
        );

        // message
        let msg_dim = measure_text(msg, None, 28, 1.0);
        draw_text(
            msg,
            w * 0.5 - msg_dim.width * 0.5,
            h * 0.45,
            28.0,
            WHITE,
        );

        let hint = "Press R to reset";
        let hint_dim = measure_text(hint, None, 24, 1.0);
        draw_text(
            hint,
            w * 0.5 - hint_dim.width * 0.5,
            h * 0.6,
            24.0,
            GRAY,
        );
    }
}
