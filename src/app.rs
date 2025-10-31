use crate::nes::NES;
use macroquad::prelude::*;
use tinyaudio::prelude::*;

use std::sync::atomic::{AtomicU8, Ordering};
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::sync::Arc;
use crate::display::color_map::SYSTEM_PALETTE;
use crate::error::{EmulatorError, EmulatorErrorType};
use crate::nes::controller::joypad::JoypadButton;

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

pub struct EmulatorApp {
    nes_arc: Arc<NesCell>,
    audio_device: Option<OutputDevice>,
    key_map: HashMap<KeyCode, JoypadButton>,

    pixel_buffer: Vec<u8>,
    texture: Option<Texture2D>,
}

impl EmulatorApp {
    /// Called once before the first frame.
    pub fn new() -> Self {
        let mut nes = NES::new();
        nes.set_sample_frequency(44_100);
        let nes_arc = NesCell::new(nes);
        let nes_audio = nes_arc.clone();

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

        let mut keycode_to_joypad: HashMap<KeyCode, JoypadButton> = HashMap::new();
        for (keycode, button) in key_map_data.iter() {
            keycode_to_joypad.insert(*keycode, *button);
        }
        
        // let pixel_buffer = vec![egui::Color32::BLACK; 256 * 240];

        Self {
            nes_arc,
            audio_device: None,
            key_map: keycode_to_joypad,
            
            pixel_buffer: vec![0;240*256],
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
    
    pub fn init_audio(&mut self) -> Result<(), EmulatorError>{
        let nes_clone = self.nes_arc.clone();
        let audio_device = run_output_device(
            OutputDeviceParameters {
                channels_count: 1,
                sample_rate: 44_100,
                channel_sample_count: 4410,
            },
            move |data| {
                unsafe {
                    let nes = nes_clone.get_mut();
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
        );
        
        match audio_device {
            Ok(audio_device) => {
                self.audio_device = Some(audio_device);
                Ok(())
            },
            _ => {
                self.audio_device = None;
                Err(EmulatorError::new(EmulatorErrorType::AudioInitFailed, "init_audio()".to_string()))
            }
        }
    }
    
    pub async fn run(&mut self) {
        // TODO: Handle state
        enum State {
            Start,
            InitAudio,
            Running,
            Stopped,
            Error
        }
        
        let mut state = State::Start;
        loop {
            match state {
                State::Start => {
                    state = State::InitAudio;
                }
                State::InitAudio => {
                    if self.audio_device.is_none() {
                        if let Err(e) = self.init_audio() {
                            state = State::Error;
                        }
                    } else {
                        state = State::Running;
                    }
                }
                State::Running => self.run_emulation().await,
                State::Stopped => {}
                State::Error => {
                    panic!("Error happened...");
                }
            }
        }
    }
    
    pub async fn run_emulation(&mut self) {
        let mut last_frame_time = get_time();
        
        loop {
            let current_time = get_time();
            
            let delta_time = current_time - last_frame_time;
            
            self.handle_input();
            self.render();
            
            let fps = format!("FPS: {}", (1.0 / delta_time) as usize);
            draw_text(&fps, 5.0, 48.0, 24.0, Color::new(1.0, 1.0, 0.0, 1.0)); 
            
            last_frame_time = current_time;
            next_frame().await;
        }
    }

    pub fn handle_input(&mut self) {
        // Handle user input
        let keys_down = get_keys_down();
        for (key, button) in self.key_map.iter() {
            let mut pressed = false;
                if keys_down.contains(&key) {
                    // pressed = true;
                    CONTROLLER1.buttons.fetch_or((*button).bits(), Ordering::Relaxed);
                } else {
                    CONTROLLER1.buttons.fetch_and(!(*button).bits(), Ordering::Relaxed);
                    
                }
        }
    }

    pub fn render(&mut self) {
        // SAFETY: only the audio thread mutates the NES (while running)
        let nes: &NES = unsafe { self.nes_arc.get_ref() };
        
        let frame_buffer = nes.get_frame_buffer(); // &[u8; 256*240]
        let width = 256;
        let height = 240;

        // convert palette indices to RGBA
        // Allocate once and reuse for speed
        if self.pixel_buffer.len() != width * height * 4 {
            self.pixel_buffer.resize(width * height * 4, 0);
        }

        for (i, &p) in frame_buffer.iter().enumerate() {
            let color = SYSTEM_PALETTE[p as usize]; // spicy!
            let base = i * 4;
            self.pixel_buffer[base + 0] = color.0;
            self.pixel_buffer[base + 1] = color.1;
            self.pixel_buffer[base + 2] = color.2;
            self.pixel_buffer[base + 3] = 255; // alpha
        }

        // Create/update GPU texture
        if self.texture.is_none() {
            self.texture = Some(Texture2D::from_rgba8(width as u16, height as u16, &self.pixel_buffer));
            self.texture.as_ref().unwrap().set_filter(FilterMode::Nearest);
        } else {
            self.texture.as_ref().unwrap().update_from_bytes(
                width as u32,
                height as u32,
                &self.pixel_buffer);
        }

        // clear_background(BLACK);
        
        // Scale texture to screen
        let tex = self.texture.as_ref().unwrap();
        let screen_w = screen_width();
        let screen_h = screen_height();

        draw_texture_ex(
            tex,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(screen_w, screen_h)),
                ..Default::default()
            },
        );
    }
}
