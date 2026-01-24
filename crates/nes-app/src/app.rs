pub use crate::event::AppEvent;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::error::Error;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;

use cpal::Stream;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use egui::{ColorImage, TextureHandle, TextureOptions};

pub use crate::command::{AppCommand, AppControl};
pub use crate::event::AppEventSource;
use nes_core::prelude::*;

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
/// - Framebuffer reads may race with PPU writes (visual tearing allowed).
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

#[derive(Clone)]
pub struct AppRunState {
    paused: Arc<AtomicBool>,
}

impl AppRunState {
    pub fn new() -> Self {
        Self {
            paused: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::SeqCst);
    }
}

enum State {
    // NeedUserInteraction,
    Waiting,
    Running,
    Paused,
    Error(String),
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
    run_state: AppRunState,
    control: AppControl<AppCommand>,
    events: E,
}

impl<E: AppEventSource> App<E> {
    pub fn new(events: E) -> Self {
        Self::new_with_autostart(events, false, [])
    }

    pub fn new_with_autostart(
        events: E,
        skip_user_interaction: bool,
        initial_commands: impl IntoIterator<Item = AppCommand>,
    ) -> Self {
        let nes = NES::new();
        let nes_arc = NesCell::new(nes);
        let (tx, rx) = crossbeam_channel::unbounded();

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
        // let initial_state = if skip_user_interaction {
        //     State::Waiting
        // } else {
        //     State::NeedUserInteraction
        // };
        let initial_state = State::Waiting;

        let app = Self {
            nes_arc,
            key_map,
            texture: None,
            audio_stream: None,
            state: initial_state,
            show_debug: false,
            user_interacted: skip_user_interaction,
            run_state: AppRunState::new(),
            events,
            control: AppControl::new(tx, rx),
            log_callback: None,
        };

        for cmd in initial_commands {
            app.control.send(cmd).ok();
        }
        app
    }

    pub fn with_logger<F>(mut self, f: F) -> Self
    where
        F: Fn(String) + 'static,
    {
        self.log_callback = Some(Box::new(f));
        self
    }

    fn handle_events(&mut self) {
        while let Some(event) = self.events.poll_event() {
            self.handle_event(event);
        }
    }

    fn handle_event(&mut self, event: AppEvent) {
        self.log(format!("[App received event]: {:?}", event));
        match event {
            AppEvent::LoadRom(rom) => {
                self.control.send(AppCommand::LoadRom(rom)).ok();
            }
            AppEvent::Reset => {
                self.control.send(AppCommand::Reset).ok();
            }
            AppEvent::Pause => {
                self.control.send(AppCommand::Pause(true)).ok();
            }
        }
    }

    pub fn handle_commands(&mut self) {
        while let Ok(cmd) = self.control.receive() {
            match cmd {
                AppCommand::LoadRom(rom) => {
                    self.init_audio().ok();
                    let nes: &mut NES = unsafe { &mut *self.nes_arc.get_mut() };

                    match Rom::new(&rom).and_then(|r| r.into_cartridge()) {
                        Ok(cart) => {
                            nes.insert_cartridge(cart);
                            self.state = State::Running;
                            self.set_paused(false);
                        }
                        Err(e) => self.state = State::Error(e.to_string()),
                    }
                }
                AppCommand::Reset => self.reset(),
                AppCommand::Pause(p) => self.set_paused(p),
            }
        }
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.run_state.set_paused(paused);
        self.state = match paused {
            true => State::Paused,
            false => State::Running,
        }
    }

    pub fn init_audio(&mut self) -> Result<(), Box<dyn Error>> {
        if self.audio_stream.is_some() {
            return Ok(());
        }

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("No output device available")?;

        let config = device.default_output_config()?;
        let nes_clone = self.nes_arc.clone();
        let run_state = self.run_state.clone();

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                self.build_stream::<f32>(&device, &config.into(), nes_clone, run_state)?
            }
            cpal::SampleFormat::I16 => {
                self.build_stream::<i16>(&device, &config.into(), nes_clone, run_state)?
            }
            cpal::SampleFormat::U16 => {
                self.build_stream::<u16>(&device, &config.into(), nes_clone, run_state)?
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
        run_state: AppRunState,
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
                // if PAUSE_EMULATION.load(Ordering::SeqCst) {
                if run_state.is_paused() {
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
                        self.set_paused(true);
                    }
                    State::Paused => {
                        self.set_paused(false);
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
                // TextureOptions::NEAREST, // Pixel-perfect scaling
                TextureOptions::LINEAR,
            )
        });

        // texture.set(color_image, TextureOptions::NEAREST);
        texture.set(color_image, TextureOptions::LINEAR);

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
                                    self.control.send(AppCommand::LoadRom(rom_data)).ok();
                                }
                                ui.add_space(10.0);
                            }

                            ui.label("Or drag and drop a ROM file");
                        });
                    });
                });
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

                            ui.label(egui::RichText::new(msg).color(egui::Color32::RED));
                            ui.add_space(10.0);

                            ui.label("Press R to reset");
                            ui.add_space(20.0);

                            if ui.button("Copy Error to Clipboard").clicked() {
                                ctx.copy_text(msg.clone());
                            }
                        });
                    });
                });
            }
        }

        self.handle_file_drop(ctx);
    }

    fn handle_file_drop(&mut self, ctx: &egui::Context) {
        // Preview hovering files
        if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
            use egui::*;

            let painter =
                ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

            let content_rect = ctx.content_rect();
            painter.rect_filled(content_rect, 0.0, Color32::from_black_alpha(192));
            painter.text(
                content_rect.center(),
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
                        self.state = State::Waiting;
                        let load_event = AppEvent::LoadRom(rom_data);
                        self.handle_event(load_event);
                    }
                }

                #[cfg(target_arch = "wasm32")]
                {
                    if let Some(bytes) = &file.bytes {
                        self.state = State::Waiting;
                        let load_event = AppEvent::LoadRom(bytes.to_vec());
                        self.handle_event(load_event);
                    }
                }
            }
        });
    }

    fn render_debug_window(&self, ctx: &egui::Context) {
        egui::Window::new("Debug Info")
            .default_width(300.0)
            .show(ctx, |ui| {
                let nes: &NES = unsafe { &*self.nes_arc.get_ref() };
                ui.label(format!("PC: ${:04X}", nes.bus.cpu.program_counter));
                ui.label(format!("A: ${:02X}", nes.bus.cpu.register_a));
                ui.label(format!("X: ${:02X}", nes.bus.cpu.register_x));
                ui.label(format!("Y: ${:02X}", nes.bus.cpu.register_y));
                ui.label(format!("SP: ${:02X}", nes.bus.cpu.stack_pointer));

                ui.separator();

                ui.label("Mute Audio Channels:");
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
        let nes: &mut NES = unsafe { &mut *self.nes_arc.get_mut() };
        nes.bus.reset_components();

        self.state = State::Waiting;
    }

    fn log(&self, msg: impl Into<String>) {
        if let Some(cb) = &self.log_callback {
            cb(msg.into())
        }
    }
}

impl<E: AppEventSource> eframe::App for App<E> {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_events();
        self.handle_commands();
        self.handle_input(ctx);
        self.render_ui(ctx);

        ctx.request_repaint();
    }
}
