pub use crate::event::AppEvent;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

pub use crate::command::{AppCommand, AppControl};
pub use crate::event::AppEventSource;
use crate::snapshot::{CpuSnapshot, DebugSnapshot, FrameSnapshot};
use cpal::Stream;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{Receiver, Sender};
use egui::{ColorImage, TextureHandle, TextureOptions};
use nes_core::nes::RunState;
use nes_core::prelude::*;
use crate::controller::ControllerState;

pub struct SharedInput {
    buttons: AtomicU8,
}

enum State {
    // NeedUserInteraction,
    Waiting,
    Running,
    Paused,
    Error(String),
}

pub struct App<E: AppEventSource> {
    key_map: HashMap<egui::Key, JoypadButton>,
    texture: Option<TextureHandle>,
    audio_stream: Option<Stream>,
    state: State,
    show_debug: bool,
    user_interacted: bool,

    controller1: Arc<ControllerState>,
    controller2: Arc<ControllerState>,

    log_callback: Option<Box<dyn Fn(String) + 'static>>,
    events: E,

    // UI -> audio
    control: AppControl<AppCommand>,

    // audio -> UI
    frame_rx: Receiver<FrameSnapshot>,
    debug_rx: Receiver<DebugSnapshot>,
    log_rx: Receiver<String>,

    // stored until audio init
    cmd_rx: Option<Receiver<AppCommand>>,
    frame_tx: Option<Sender<FrameSnapshot>>,
    debug_tx: Option<Sender<DebugSnapshot>>,
    log_tx: Option<Sender<String>>,

    // received state
    latest_frame: Option<FrameSnapshot>,
    latest_debug: Option<DebugSnapshot>,
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
        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
        let (frame_tx, frame_rx) = crossbeam_channel::bounded(1);
        let (debug_tx, debug_rx) = crossbeam_channel::bounded(1);
        let (log_tx, log_rx) = crossbeam_channel::unbounded();

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
            key_map,
            texture: None,
            audio_stream: None,
            state: initial_state,
            show_debug: false,
            user_interacted: skip_user_interaction,
            events,
            log_callback: None,

            controller1: Arc::new(ControllerState::default()),
            controller2: Arc::new(ControllerState::default()),

            control: AppControl::new(cmd_tx),
            frame_rx,
            debug_rx,
            log_rx,
            cmd_rx: Some(cmd_rx),
            frame_tx: Some(frame_tx),
            debug_tx: Some(debug_tx),
            log_tx: Some(log_tx),

            latest_frame: None,
            latest_debug: None,
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
            AppEvent::RequestLoadRom(rom) => {
                self.init_audio().ok();
                self.state = State::Running;
                self.control.load_rom(rom);
            }
            AppEvent::RequestReset => {
                self.control.reset();
                self.state = State::Waiting;
            }
            AppEvent::RequestPause => {
                let paused = !matches!(self.state, State::Paused);
                self.control.pause(paused);
                self.state = if paused {
                    State::Paused
                } else {
                    State::Running
                };
            }
        }
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.control.pause(paused);
    }

    pub fn init_audio(&mut self) -> Result<(), Box<dyn Error>> {
        if self.audio_stream.is_some() {
            return Ok(());
        }

        let nes = NES::new();
        let controller1 = Arc::clone(&self.controller1);
        let controller2 = Arc::clone(&self.controller2);

        let cmd_rx = self.cmd_rx.take().expect("audio already initialized");
        let frame_tx = self.frame_tx.take().expect("audio already initialized");
        let debug_tx = self.debug_tx.take().expect("audio already initialized");
        let log_tx = self.log_tx.take().expect("audio already initialized");

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("No output device available")?;
        let config = device.default_output_config()?;

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => self.build_stream::<f32>(
                &device,
                &config.into(),
                nes,
                controller1,
                controller2,
                cmd_rx,
                frame_tx,
                debug_tx,
                log_tx,
            )?,
            cpal::SampleFormat::I16 => self.build_stream::<i16>(
                &device,
                &config.into(),
                nes,
                controller1,
                controller2,
                cmd_rx,
                frame_tx,
                debug_tx,
                log_tx,
            )?,
            cpal::SampleFormat::U16 => self.build_stream::<u16>(
                &device,
                &config.into(),
                nes,
                controller1,
                controller2,
                cmd_rx,
                frame_tx,
                debug_tx,
                log_tx,
            )?,
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
        mut nes: NES,
        controller1: Arc<ControllerState>,
        controller2: Arc<ControllerState>,
        cmd_rx: Receiver<AppCommand>,
        frame_tx: Sender<FrameSnapshot>,
        debug_tx: Sender<DebugSnapshot>,
        log_tx: Sender<String>,
    ) -> Result<Stream, Box<dyn Error>>
    where
        T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>,
    {
        let sample_rate = config.sample_rate as f64;
        let channels = config.channels as usize;

        let stream = device.build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                // Handle commands
                while let Ok(cmd) = cmd_rx.try_recv() {
                    match cmd {
                        AppCommand::LoadRom(rom) => {
                            log_tx
                                .try_send(format!("[Audio] LoadRom received ({} bytes)", rom.len()))
                                .ok();
                            if let Ok(cart) = Rom::new(&rom).and_then(|r| r.into_cartridge()) {
                                nes.insert_cartridge(cart);
                                nes.run_state = RunState::Running;
                            }
                        }
                        AppCommand::Reset => {
                            nes.bus.reset_components();
                        }
                        AppCommand::Pause(paused) => {
                            nes.run_state = if paused {
                                RunState::Paused
                            } else {
                                RunState::Running
                            };
                        }
                        AppCommand::SetApuMute { .. } => {}
                    }
                }

                // Handle run state audio
                match nes.run_state {
                    RunState::Paused => {
                        for sample in data.iter_mut() {
                            *sample = T::from_sample(0.0f32);
                        }
                    }
                    RunState::Running => {
                        log_tx.try_send("RUNNING".to_string()).ok();
                        // PPU cycles per audio sample (5.369318 MHz / 44.1 kHz)
                        let mut frame_ready = false;
                        let ppu_cycles_per_sample = 5369318.0 / sample_rate;
                        let mut cycle_acc = nes.cycle_acc;

                        let p1 = controller1.load();
                        let p2 = controller2.load();
                        nes.bus.joypads[0].set_buttons(p1);
                        nes.bus.joypads[1].set_buttons(p2);

                        for frame in data.chunks_mut(channels) {
                            cycle_acc += ppu_cycles_per_sample;

                            while cycle_acc >= 1.0 {
                                if nes.tick() {
                                    frame_ready = true;
                                }
                                cycle_acc -= 1.0;
                            }

                            let raw = nes.bus.apu.sample();
                            let sample = T::from_sample(raw);

                            for out in frame.iter_mut() {
                                *out = sample;
                            }
                        }
                        nes.cycle_acc = cycle_acc;

                        // Send snapshots
                        if frame_ready {
                            let _ = frame_tx.try_send(FrameSnapshot {
                                pixels: nes.get_frame_buffer().to_vec(),
                            });
                            let _ = debug_tx.try_send(DebugSnapshot {
                                cpu: CpuSnapshot {
                                    program_counter: nes.bus.cpu.program_counter,
                                    register_a: nes.bus.cpu.register_a,
                                    register_x: nes.bus.cpu.register_x,
                                    register_y: nes.bus.cpu.register_y,
                                    stack_pointer: nes.bus.cpu.stack_pointer,
                                    status: nes.bus.cpu.status.bits(),
                                },
                            });
                        }
                    }
                }
            },
            |err| eprintln!("Audio stream error: {}", err),
            None,
        )?;
        Ok(stream)
    }

    fn handle_input(&mut self, ctx: &egui::Context) {
        // Handle controller input
        let mut p1 = 0u8;
        for (key, button) in self.key_map.iter() {
            if ctx.input(|i| i.key_down(*key)) {
                p1 |= button.bits();
            }
        }
        self.controller1.set(p1);
        // TODO: Add support for controller 2

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
            // let nes: &mut NES = unsafe { &mut *self.nes_arc.get_mut() };
            // if i.key_pressed(egui::Key::Num1) {
            //     nes.bus.apu.mute_pulse1 = !nes.bus.apu.mute_pulse1;
            // }
            // if i.key_pressed(egui::Key::Num2) {
            //     nes.bus.apu.mute_pulse2 = !nes.bus.apu.mute_pulse2;
            // }
            // if i.key_pressed(egui::Key::Num3) {
            //     nes.bus.apu.mute_triangle = !nes.bus.apu.mute_triangle;
            // }
            // if i.key_pressed(egui::Key::Num4) {
            //     nes.bus.apu.mute_noise = !nes.bus.apu.mute_noise;
            // }
            // if i.key_pressed(egui::Key::Num5) {
            //     nes.bus.apu.mute_dmc = !nes.bus.apu.mute_dmc;
            // }
        });
    }

    fn render_display(&mut self, ui: &mut egui::Ui) {
        if let Ok(frame) = self.frame_rx.try_recv() {
            self.latest_frame = Some(frame);
        }
        if let Ok(debug) = self.debug_rx.try_recv() {
            self.latest_debug = Some(debug);
        }

        // Convert NES framebuffer to egui's ColorImage
        if let Some(frame) = &self.latest_frame {
            let mut pixels = Vec::with_capacity(256 * 240 * 4);
            for &palette_idx in frame.pixels.iter() {
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
                        let msg = format!("Received drop file event. ({} bytes)", rom_data.len());
                        self.log(msg);
                        self.init_audio().ok();
                        self.control.load_rom(rom_data);
                        self.state = State::Running;
                    }
                }

                #[cfg(target_arch = "wasm32")]
                {
                    if let Some(bytes) = &file.bytes {
                        self.state = State::Waiting;
                        let load_event = AppEvent::RequestLoadRom(bytes.to_vec());
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
                // let nes: &NES = unsafe { &*self.nes_arc.get_ref() };
                // ui.label(format!("PC: ${:04X}", nes.bus.cpu.program_counter));
                // ui.label(format!("A: ${:02X}", nes.bus.cpu.register_a));
                // ui.label(format!("X: ${:02X}", nes.bus.cpu.register_x));
                // ui.label(format!("Y: ${:02X}", nes.bus.cpu.register_y));
                // ui.label(format!("SP: ${:02X}", nes.bus.cpu.stack_pointer));
                //
                // ui.separator();
                //
                // ui.label("Mute Audio Channels:");
                // ui.checkbox(
                //     &mut unsafe { &mut *self.nes_arc.get_mut() }.bus.apu.mute_pulse1,
                //     "Pulse 1",
                // );
                // ui.checkbox(
                //     &mut unsafe { &mut *self.nes_arc.get_mut() }.bus.apu.mute_pulse2,
                //     "Pulse 2",
                // );
                // ui.checkbox(
                //     &mut unsafe { &mut *self.nes_arc.get_mut() }
                //         .bus
                //         .apu
                //         .mute_triangle,
                //     "Triangle",
                // );
                // ui.checkbox(
                //     &mut unsafe { &mut *self.nes_arc.get_mut() }.bus.apu.mute_noise,
                //     "Noise",
                // );
                // ui.checkbox(
                //     &mut unsafe { &mut *self.nes_arc.get_mut() }.bus.apu.mute_dmc,
                //     "DMC",
                // );
            });
    }
    fn reset(&mut self) {
        self.control.reset();
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
        self.handle_input(ctx);
        self.render_ui(ctx);

        while let Ok(msg) = self.log_rx.try_recv() {
            self.log(msg);
        }

        ctx.request_repaint();
    }
}
