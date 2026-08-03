use crate::app::action::Action;
use crate::app::event::{AppEvent, AppEventSource};
pub(crate) use crate::app::ui::app_input;
use crate::app::ui::error::ErrorInfo;
use crate::app::ui::file_drop_overlay;
use crate::app::ui::views::UiView;
use crate::app::ui::views::error_view::ErrorView;
use crate::app::ui::views::rom_select_view::RomSelectView;
use crate::app::ui::views::waiting_view::WaitingView;
use crate::emu::commands::EmuCommand;
use crate::emu::event::EmuEvent;
use crate::emu::host::EmuHost;
use crate::shared::frame_buffer::{SharedFrame, SharedFrameHandle};
use anyhow::Context;
use eframe::epaint::TextureHandle;
use nes_core::prelude::Rom;
use std::sync::{Arc, Mutex};
use crate::app::ui::post_fx::PostFx;

pub struct UiCtx<'a> {
    pub frame: &'a SharedFrameHandle,
    pub post_fx: Arc<Mutex<PostFx>>,
    // pub texture: &'a mut Option<TextureHandle>,
    pub actions: &'a mut Vec<Action>,
    pub started: bool,
    pub paused: bool,
    pub time_s: f32,
}

pub struct App<E: AppEventSource> {
    pub(crate) events: E,
    emu_host: Option<EmuHost>,
    pub(crate) frame: SharedFrameHandle,

    pub(crate) post_fx: Arc<Mutex<PostFx>>,
    // pub(crate) texture: Option<TextureHandle>,
    log_callback: Option<Box<dyn Fn(String) + 'static>>,

    // UI
    pub(crate) view: UiView,
    pub(crate) started: bool,
    pub(crate) paused: bool,
}

impl<E: AppEventSource> App<E> {
    pub fn new(cc: &eframe::CreationContext<'_>, events: E) -> Self {
        let gl = cc.gl.as_ref().expect("glow backend required cc.gl").clone();
        let post_fx = Arc::new(Mutex::new(unsafe { PostFx::new(gl) }));

        Self {
            events,
            emu_host: None,
            frame: Arc::new(SharedFrame::new()),
            post_fx,
            // texture: None,


            log_callback: None,
            view: UiView::Waiting(WaitingView::new()),
            started: false,
            paused: false,
        }
    }

    /// When building to WASM, this must be called in a user-interaction context
    pub fn start_emulator(&mut self) {
        if self.started {
            self.log("App::start_emulator() already called.");
            return;
        }
        self.started = true;

        if self.emu_host.is_some() {
            panic!("Double App::start() shouldn't happen");
        }
        self.log("App::start()");
        match EmuHost::start(self.frame.clone()) {
            Ok(emu) => {
                self.log("EmuHost::start() => Ok()");
                self.emu_host = Some(emu);
                self.apply_action(Action::Navigate(UiView::RomSelect(RomSelectView::new())))
            }
            Err(e) => {
                self.log(format!("EmuHost::start() => Err(): {}", e));
                self.set_error(e);
            }
        }
    }

    pub fn with_initial_events(mut self, events: impl IntoIterator<Item = AppEvent>) -> Self {
        for event in events {
            if let Err(e) = self.handle_external_event(event) {
                self.set_error(e);
                break;
            }
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

    /// Handle events from the Emulator Runtime
    fn handle_emu_events(&mut self) {
        let Some(emu) = self.emu_host.as_ref() else {
            return;
        };
        while let Some(event) = emu.try_recv() {
            match event {
                EmuEvent::Log(msg) => {
                    self.log(msg);
                }
            }
        }
    }

    /// Send commands to the Emulator Runtime
    pub(crate) fn send_command(&self, cmd: EmuCommand) {
        if let Some(emu) = &self.emu_host {
            emu.send(cmd);
        }
    }

    pub fn log(&self, message: impl Into<String>) {
        if let Some(callback) = self.log_callback.as_ref() {
            callback(message.into());
        }
    }

    fn set_error(&mut self, error: anyhow::Error) {
        let info = ErrorInfo::from_anyhow("", error);
        self.apply_action(Action::SetPaused(true));
        self.view = UiView::Error(ErrorView::new(info));
    }

    fn load_rom_and_start(&mut self, rom_bytes: Vec<u8>) -> anyhow::Result<()> {
        let rom = Rom::parse(&rom_bytes).context("Rom parsing failed")?;
        let cartridge = rom.into_cartridge().context("Cartridge parsing failed")?;
        self.log("Cartridge parsed!");
        self.send_command(EmuCommand::InsertCartridge(cartridge));
        Ok(())
    }

    pub(crate) fn play_rom(&mut self, rom_bytes: Vec<u8>) {
        match self.load_rom_and_start(rom_bytes) {
            Ok(()) => self.view = UiView::playing(),
            Err(e) => self.set_error(e),
        }
    }
}

impl<E: AppEventSource> eframe::App for App<E> {
    /// Serves as the main UI loop
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle External events received from the browser (for WASM builds)
        if let Err(e) = self.handle_external_events() {
            self.set_error(e);
            return;
        }
        self.handle_emu_events();

        // Update controller states while playing
        if matches!(self.view, UiView::Playing { .. })
            && let Some(emu_host) = self.emu_host.as_ref()
        {
            app_input::update_controller_state(ctx, &emu_host);
        }

        let mut actions = Vec::<Action>::new();
        let time_s = ctx.input(|i| i.time as f32);
        {
            // Build context
            let mut ui_ctx = UiCtx {
                frame: &self.frame,
                // texture: &mut self.texture,
                post_fx: self.post_fx.clone(),
                actions: &mut actions,
                started: self.started,
                paused: self.paused,
                time_s,
            };

            // Handle Hotkeys
            app_input::handle_hotkeys(ctx, &mut ui_ctx, &self.view);

            // Render
            match &mut self.view {
                UiView::RomSelect(v) => v.ui(ctx, &mut ui_ctx),
                UiView::Options => {}
                UiView::Playing(v) => v.ui(ctx, &mut ui_ctx),
                UiView::Error(v) => v.ui(ctx, &mut ui_ctx),
                UiView::Waiting(v) => v.ui(ctx, &mut ui_ctx),
            }

            // Allow file-drop only if emulator has already started
            if self.started {
                file_drop_overlay::handle_file_drop(ctx, &mut ui_ctx);
            }
        };

        // Commit actions
        self.apply_actions(actions);

        ctx.request_repaint();
    }
}
