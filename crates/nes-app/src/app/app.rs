use std::sync::Arc;
use eframe::epaint::TextureHandle;
use nes_core::nes::RunState::Paused;
use nes_core::prelude::{Cartridge, Rom, RomError};
use crate::app::event::{AppEvent, AppEventSource};
use crate::app::ui::main_view;
use crate::app::ui::app_input;
use crate::emu::host::EmuHost;
use crate::shared::frame_buffer::{SharedFrameHandle};

pub struct App<E: AppEventSource> {
    events: E,
    emu: Option<EmuHost>,
    frame: Option<SharedFrameHandle>,
    texture: Option<TextureHandle>,
    log_callback: Option<Box<dyn Fn(String) + 'static>>,

    // UI
    state: UiState,
    show_debug: bool,
    last_error: Option<String>
}

struct UiState {
    paused: bool,
    user_interacted: bool,
}

impl<E: AppEventSource> App<E> {
    pub fn new(events: E) -> Self {
        Self {
            events,
            emu: None,
            frame: None,
            texture: None,

            log_callback: None,
            state: UiState { paused: false, user_interacted: false },
            show_debug: false,
            last_error: None,
        }
    }

    pub fn start(&mut self) {
        if self.emu.is_some() {
            panic!("Double App::start() shouldn't happen");
        }
        self.log("App::start()");
        match EmuHost::start() {
            Ok((emu, frame)) => {
                self.emu = Some(emu);
                self.frame = Some(frame);
            },
            Err(e) => {
                self.last_error = Some(e.to_string());
            }
        }
    }

    pub fn with_initial_events(mut self, events: impl IntoIterator<Item = AppEvent>) -> Self {
        for event in events {
            self.handle_external_event(event);
        }
        self
    }

    pub fn with_logger<F>(mut self, f: F) -> Self
    where
        F: Fn(String) + 'static,
    {
        self.log_callback = Some(Box::new(f));
        self
    }

    fn handle_external_events(&mut self) {
        while let Some(event) = self.events.poll_event() {
            self.log("[RECEIVED] handle_external_events()");
            self.handle_external_event(event);
        }
    }

    fn handle_external_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::Start => self.start(),
            AppEvent::LoadRom(rom) => {
                self.log("AppEvent::LoadRom");
                match Rom::parse(&rom) {
                    Ok(rom) => {
                        match rom.into_cartridge() {
                            Ok(cartridge) => {}
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
            _ => {
                self.log(format!("\t[Unhandled AppEvent] {:?}", event));
            }
        }
    }

    fn handle_emu_events(&mut self) {
        let Some(emu) = self.emu.as_ref() else {
            return;
        };
        while let Some(event) = emu.try_recv() {
            self.log("[RECEIVED] handle_emu_events()");
            match event {
                _ => {}
            }
        }
    }

    pub fn log(&self, message: impl Into<String>) {
        if let Some(callback) = self.log_callback.as_ref() {
            callback(message.into());
        }
    }

    fn set_error(&mut self, error: String) {
        self.state.paused = true;
        self.last_error = Some(error);
    }
}

impl<E: AppEventSource> eframe::App for App<E> {
    /// Serves as the main UI loop
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_external_events();
        self.handle_emu_events();

        if let (Some(emu), Some(frame)) = (self.emu.as_ref(), self.frame.as_ref()) {
            app_input::update_controller_state(ctx, &emu);

            main_view::render(ctx, &mut self.texture, &frame, self.state.paused);

            if self.show_debug {
                // debug ui
            }
        } else {
            // TODO: render waiting ui
        }

        ctx.request_repaint();
    }
}