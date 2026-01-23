use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::error::Error;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

use cpal::Stream;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use egui::{ColorImage, TextureHandle, TextureOptions};

use nes_core::prelude::*;
// use tinyaudio::prelude::*;

use crate::display::consts::{WINDOW_HEIGHT, WINDOW_WIDTH};

/// Wrapper around NES that allows shared access across threads.
///
/// The NES is driven primarily by the audio thread during emulation.
/// The Main thread may temporarily take mutable access while emulation is paused.
/// When emulation is running the Main thread only reads immutable state.
///
/// Synchronization is coordinated via atomic flags:
/// - PAUSE_EMULATION gates audio-thread execution
/// - Reset/load only occur while paused
pub struct NesCell(UnsafeCell<NES>);

/// SAFETY:
/// - NES is not thread-safe by itself.
/// - At most one thread mutates NES at a time.
/// - Audio thread mutates NES only while PAUSE_EMULATION == false.
/// - UI thread mutates NES only while PAUSE_EMULATION == true.
/// - Render thread only performs immutable reads.
/// - Framebuffer reads may race with PPU writes (visual tearing allowed).
/// - Atomic flags coordinate mutation phases.
///
/// This makes Send + Sync sound under the documented execution model.
unsafe impl Send for NesCell {}
unsafe impl Sync for NesCell {}

impl NesCell {
    pub fn new(nes: NES) -> Arc<Self> {
        Arc::new(Self(UnsafeCell::new(nes)))
    }

    /// Get mutable access to NES.
    ///
    /// # SAFETY
    /// Caller must guarantee:
    /// - Emulation is paused OR this is the audio thread
    /// - No other mutable access is active
    /// - No immutable access is active during mutation (except for rendering purposes)
    #[inline(always)]
    pub unsafe fn get_mut(&self) -> *mut NES {
        self.0.get()
    }

    /// Get immutable access to NES.
    ///
    /// # SAFETY
    /// Caller must guarantee:
    /// - This is only used for read-only inspection (rendering, error display)
    #[inline(always)]
    pub unsafe fn get_ref(&self) -> &NES {
        unsafe { &*self.0.get() }
    }
}

pub struct SharedInput {
    buttons: AtomicU8,
}
pub static CONTROLLER1: SharedInput = SharedInput {
    buttons: AtomicU8::new(0),
};

// TODO: Potentially rename these to better reflect the "done-ness" of the respective states
pub static TRIGGER_LOAD: AtomicBool = AtomicBool::new(false);
pub static TRIGGER_RESET: AtomicBool = AtomicBool::new(false);
pub static PAUSE_EMULATION: AtomicBool = AtomicBool::new(true);
pub static ROM_DATA: Mutex<Vec<u8>> = Mutex::new(Vec::new());

enum State {
    NeedUserInteraction,
    Waiting,
    Running,
    Paused,
    Error(String),
}

#[derive(Debug)]
pub enum AppEvent {
    LoadRom(Vec<u8>),
    Reset,
    Pause,
}

pub trait AppEventSource {
    fn poll_event(&mut self) -> Option<AppEvent>;
}

pub struct App<E: AppEventSource> {
    nes_arc: Arc<NesCell>,
    key_map: HashMap<egui::Key, JoypadButton>,
    texture: Option<TextureHandle>,
    audio_stream: Option<Stream>,
    state: State,
    show_debug: bool,
    user_interacted: bool,

    log_callback: Option<Box<dyn Fn(String) + 'static>>,
    events: E,
}

impl<E: AppEventSource> App<E> {
    pub fn new(events: E) -> Self {
        Self::new_with_autostart(events, false)
    }

    pub fn new_with_autostart(events: E, skip_user_interaction: bool) -> Self {
        let nes = NES::new();
        let nes_arc = NesCell::new(nes);

        let key_map_data: &[(egui::Key, JoypadButton)] = &[
            (egui::Key::K, JoypadButton::BUTTON_A),
            (egui::Key::J, JoypadButton::BUTTON_B),
            (egui::Key::Enter, JoypadButton::START),
            (egui::Key::Space, JoypadButton::SELECT),
            (egui::Key::W, JoypadButton::UP),
            (egui::Key::S, JoypadButton::DOWN),
            (egui::Key::A, JoypadButton::LEFT),
            (egui::Key::D, JoypadButton::RIGHT),
        ];

        let mut key_map = HashMap::new();
        for (k, v) in key_map_data.iter() {
            key_map.insert(*k, *v);
        }

        // Skip user interaction screen for native with ROM argument
        let initial_state = if skip_user_interaction {
            State::Waiting
        } else {
            State::NeedUserInteraction
        };

        Self {
            nes_arc,
            key_map,
            texture: None,
            audio_stream: None,
            state: initial_state,
            show_debug: false,
            user_interacted: skip_user_interaction,
            events,
            log_callback: None,
        }
    }

    pub fn with_logger<F>(mut self, f: F) -> Self
    where
        F: Fn(String) + 'static,
    {
        self.log_callback = Some(Box::new(f));
        self
    }

    fn handle_event(&mut self, event: AppEvent) {
        self.log(format!("[App received event]: {:?}", event));
        match event {
            AppEvent::LoadRom(rom) => {
                TRIGGER_RESET.store(true, Ordering::SeqCst);
                *ROM_DATA.lock().unwrap() = rom;
                TRIGGER_LOAD.store(true, Ordering::SeqCst);
            }
            AppEvent::Reset => {
                TRIGGER_RESET.store(true, Ordering::SeqCst);
            }
            AppEvent::Pause => {
                PAUSE_EMULATION.store(false, Ordering::SeqCst);
                self.state = State::Running;
            }
        }
    }

    pub fn init_audio(&mut self) -> Result<(), Box<dyn Error>> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("No output device available")?;

        let config = device.default_output_config()?;
        let nes_clone = self.nes_arc.clone();

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                self.build_stream::<f32>(&device, &config.into(), nes_clone)?
            }
            cpal::SampleFormat::I16 => {
                self.build_stream::<i16>(&device, &config.into(), nes_clone)?
            }
            cpal::SampleFormat::U16 => {
                self.build_stream::<u16>(&device, &config.into(), nes_clone)?
            }
            _ => return Err("Unsupported sample format".into()),
        };

        stream.play()?;
        self.audio_stream = Some(stream);
        Ok(())
    }

    fn build_stream<T>(
        &self,
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        nes_arc: Arc<NesCell>,
    ) -> Result<Stream, Box<dyn Error>>
    where
        T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>,
    {
        let sample_rate = config.sample_rate as f64;
        let channels = config.channels as usize;

        let stream = device.build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                // if paused, send silence and don't tick NES
                if PAUSE_EMULATION.load(Ordering::SeqCst) {
                    for sample in data.iter_mut() {
                        *sample = T::from_sample(0.0f32);
                    }
                    return;
                }

                // SAFETY:
                // This closure is executed exclusively on the audio thread.
                // No other thread mutates NES while audio is running.
                // The render thread only performs immutable reads.
                let nes: &mut NES = unsafe { &mut *nes_arc.get_mut() };
                nes.bus
                    .controller1
                    .set_buttons(CONTROLLER1.buttons.load(Ordering::SeqCst));

                // PPU cycles per audio sample (5.369318 MHz / 44.1 kHz)
                let ppu_cycles_per_sample = 5369318.0 / sample_rate;
                let mut cycle_acc = nes.cycle_acc;

                for frame in data.chunks_mut(channels) {
                    cycle_acc += ppu_cycles_per_sample;

                    while cycle_acc >= 1.0 {
                        nes.tick();
                        cycle_acc -= 1.0;
                    }

                    let raw = nes.bus.apu.sample();
                    let sample = T::from_sample(raw);

                    for out in frame.iter_mut() {
                        *out = sample;
                    }
                }
                nes.cycle_acc = cycle_acc;
            },
            |err| eprintln!("Audio stream error: {}", err),
            None,
        )?;
        Ok(stream)
    }

    fn handle_input(&mut self, ctx: &egui::Context) {
        // Handle controller input
        for (key, button) in self.key_map.iter() {
            if ctx.input(|i| i.key_down(*key)) {
                CONTROLLER1
                    .buttons
                    .fetch_or(button.bits(), Ordering::SeqCst);
            } else {
                CONTROLLER1
                    .buttons
                    .fetch_and(!button.bits(), Ordering::SeqCst);
            }
        }

        // Handle other keys
        ctx.input(|i| {
            if i.key_pressed(egui::Key::P) {
                match self.state {
                    State::Running => {
                        PAUSE_EMULATION.store(true, Ordering::SeqCst);
                        self.state = State::Paused;
                    }
                    State::Paused => {
                        PAUSE_EMULATION.store(false, Ordering::SeqCst);
                        self.state = State::Running;
                    }
                    _ => {}
                }
            }

            if i.key_pressed(egui::Key::R) && matches!(self.state, State::Error(_)) {
                self.reset();
            }

            if i.key_pressed(egui::Key::F1) {
                self.show_debug = !self.show_debug;
            }

            // Mute channels
            let nes: &mut NES = unsafe { &mut *self.nes_arc.get_mut() };
            if i.key_pressed(egui::Key::Num1) {
                nes.bus.apu.mute_pulse1 = !nes.bus.apu.mute_pulse1;
            }
            if i.key_pressed(egui::Key::Num2) {
                nes.bus.apu.mute_pulse2 = !nes.bus.apu.mute_pulse2;
            }
            if i.key_pressed(egui::Key::Num3) {
                nes.bus.apu.mute_triangle = !nes.bus.apu.mute_triangle;
            }
            if i.key_pressed(egui::Key::Num4) {
                nes.bus.apu.mute_noise = !nes.bus.apu.mute_noise;
            }
            if i.key_pressed(egui::Key::Num5) {
                nes.bus.apu.mute_dmc = !nes.bus.apu.mute_dmc;
            }
        });
    }

    fn update_state(&mut self) {
        match &self.state {
            State::NeedUserInteraction => {}
            State::Paused => {}
            State::Error(msg) => {}
            State::Waiting => {
                if TRIGGER_LOAD.swap(false, Ordering::SeqCst) {
                    if self.audio_stream.is_none()
                        && let Err(e) = self.init_audio()
                    {
                        self.state = State::Error(format!("Audio init failed: {}", e));
                        return;
                    }

                    let nes: &mut NES = unsafe { &mut *self.nes_arc.get_mut() };
                    let rom_data = ROM_DATA.lock().unwrap();

                    match Rom::new(&rom_data) {
                        Ok(rom) => match rom.into_cartridge() {
                            Ok(cart) => {
                                nes.insert_cartridge(cart);
                                self.state = State::Running;
                                TRIGGER_RESET.store(false, Ordering::SeqCst);
                                PAUSE_EMULATION.store(false, Ordering::SeqCst);
                            }
                            Err(err) => {
                                self.state = State::Error(err.to_string());
                            }
                        },
                        Err(err) => {
                            self.state = State::Error(err.to_string());
                        }
                    }
                }
            }
            State::Running => {
                if TRIGGER_RESET.swap(false, Ordering::SeqCst) {
                    self.reset();
                }

                let nes: &NES = unsafe { self.nes_arc.get_ref() };
                if let Some(err) = &nes.bus.cpu.error {
                    self.state = State::Error(err.to_string());
                }
            }
        }
    }

    fn render_display(&mut self, ui: &mut egui::Ui) {
        let nes = unsafe { self.nes_arc.get_ref() };
        let frame = nes.get_frame_buffer();

        // Convert NES framebuffer to egui's ColorImage
        let mut pixels = Vec::with_capacity(256 * 240 * 4);
        for &palette_idx in frame.iter() {
            let color = NES_SYSTEM_PALETTE[palette_idx as usize];
            pixels.push(color.0); // R
            pixels.push(color.1); // G
            pixels.push(color.2); // B
            pixels.push(255); // A
        }

        let color_image = ColorImage::from_rgba_unmultiplied([256, 240], &pixels);

        // Create or update texture
        let texture = self.texture.get_or_insert_with(|| {
            ui.ctx().load_texture(
                "nes_frame",
                color_image.clone(),
                TextureOptions::NEAREST, // Pixel-perfect scaling
            )
        });

        texture.set(color_image, TextureOptions::NEAREST);

        // Display the texture, scaled to fill available space
        let available_size = ui.available_size();
        let aspect_ratio = 256.0 / 240.0;

        let (width, height) = if available_size.x / available_size.y > aspect_ratio {
            // limit by height
            (available_size.y * aspect_ratio, available_size.y)
        } else {
            // limit by width
            (available_size.x, available_size.x / aspect_ratio)
        };

        ui.image((texture.id(), egui::vec2(width, height)));
    }

    fn render_ui(&mut self, ctx: &egui::Context) {
        match &self.state {
            State::NeedUserInteraction => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.heading("NES Emulator");
                            ui.add_space(20.0);

                            if ui.button("Click to Start").clicked() {
                                self.user_interacted = true;
                                self.state = State::Waiting;
                            }

                            ui.add_space(10.0);
                            ui.label("(Required for audio to work)");
                        });
                    });
                });
            }
            State::Waiting => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.heading("Insert a Cartridge");
                            ui.add_space(10.0);

                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                if ui.button("Browse for ROM...").clicked()
                                    && let Some(path) = rfd::FileDialog::new()
                                        .add_filter("NES ROM", &["nes"])
                                        .pick_file()
                                    && let Ok(rom_data) = std::fs::read(path)
                                {
                                    *ROM_DATA.lock().unwrap() = rom_data;
                                    TRIGGER_LOAD.store(true, Ordering::SeqCst);
                                }
                                ui.add_space(10.0);
                            }

                            ui.label("Or drag and drop a ROM file");
                        });
                    });
                });

                // Handle drag and drop
                self.handle_file_drop(ctx);
            }
            State::Running | State::Paused => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    self.render_display(ui);
                });

                if matches!(self.state, State::Paused) {
                    egui::Window::new("Paused")
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .collapsible(false)
                        .resizable(false)
                        .show(ctx, |ui| {
                            ui.label("Press P to unpause");
                        });
                }

                if self.show_debug {
                    self.render_debug_window(ctx);
                }
            }
            State::Error(msg) => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.heading("Emulator Error");
                            ui.add_space(10.0);

                            // Show the actual error message
                            ui.label(egui::RichText::new(msg).color(egui::Color32::RED));

                            ui.add_space(10.0);
                            ui.label("Press R to reset");

                            // Add debug info
                            ui.add_space(20.0);
                            if ui.button("Copy Error to Clipboard").clicked() {
                                ctx.copy_text(msg.clone());
                            }
                        });
                    });
                });
            }
        }
    }

    fn handle_file_drop(&mut self, ctx: &egui::Context) {
        // Preview hovering files
        if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
            use egui::*;

            let painter =
                ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

            let screen_rect = ctx.screen_rect();
            painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
            painter.text(
                screen_rect.center(),
                Align2::CENTER_CENTER,
                "Drop ROM file here",
                FontId::proportional(40.0),
                Color32::WHITE,
            );
        }

        // Handle dropped files
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty()
                && let Some(file) = i.raw.dropped_files.first()
            {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if let Some(path) = &file.path
                        && let Ok(rom_data) = std::fs::read(path)
                    {
                        *ROM_DATA.lock().unwrap() = rom_data;
                        TRIGGER_LOAD.store(true, Ordering::SeqCst);
                    }
                }

                #[cfg(target_arch = "wasm32")]
                {
                    if let Some(bytes) = &file.bytes {
                        *ROM_DATA.lock().unwrap() = bytes.to_vec();
                        TRIGGER_LOAD.store(true, Ordering::SeqCst);
                    }
                }
            }
        });
    }

    fn render_debug_window(&self, ctx: &egui::Context) {
        egui::Window::new("Debug Info")
            .default_width(300.0)
            .show(ctx, |ui| {
                // ui.label(format!("PC: ${:04X}", nes.bus.cpu.pc));
                // ui.label(format!("A: ${:02X}", nes.bus.cpu.a));
                // ui.label(format!("X: ${:02X}", nes.bus.cpu.x));
                // ui.label(format!("Y: ${:02X}", nes.bus.cpu.y));
                // ui.label(format!("SP: ${:02X}", nes.bus.cpu.sp));

                ui.separator();

                ui.label("Audio Channels:");
                ui.checkbox(
                    &mut unsafe { &mut *self.nes_arc.get_mut() }.bus.apu.mute_pulse1,
                    "Pulse 1",
                );
                ui.checkbox(
                    &mut unsafe { &mut *self.nes_arc.get_mut() }.bus.apu.mute_pulse2,
                    "Pulse 2",
                );
                ui.checkbox(
                    &mut unsafe { &mut *self.nes_arc.get_mut() }
                        .bus
                        .apu
                        .mute_triangle,
                    "Triangle",
                );
                ui.checkbox(
                    &mut unsafe { &mut *self.nes_arc.get_mut() }.bus.apu.mute_noise,
                    "Noise",
                );
                ui.checkbox(
                    &mut unsafe { &mut *self.nes_arc.get_mut() }.bus.apu.mute_dmc,
                    "DMC",
                );
            });
    }
    fn reset(&mut self) {
        PAUSE_EMULATION.store(true, Ordering::SeqCst);
        let mut rom_data = ROM_DATA.lock().unwrap();
        *rom_data = vec![];
        TRIGGER_LOAD.store(false, Ordering::SeqCst);
        TRIGGER_RESET.store(false, Ordering::SeqCst);
        self.state = State::Waiting;

        let nes: &mut NES = unsafe { &mut *self.nes_arc.get_mut() };
        nes.bus.reset_components();
    }

    // fn set_error(&mut self, err: String) {
    //     PAUSE_EMULATION.store(true, Ordering::SeqCst);
    //     TRIGGER_RESET.store(false, Ordering::SeqCst);
    //     TRIGGER_LOAD.store(false, Ordering::SeqCst);
    //     self.state = State::Error(err.to_string());
    // }

    fn log(&self, msg: impl Into<String>) {
        if let Some(cb) = &self.log_callback {
            cb(msg.into())
        }
    }
}

impl<E: AppEventSource> eframe::App for App<E> {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Some(event) = self.events.poll_event() {
            self.handle_event(event);
        }

        self.handle_input(ctx);
        self.update_state();
        self.render_ui(ctx);

        // Request continuous repaint for smooth animation
        ctx.request_repaint();
    }
}
