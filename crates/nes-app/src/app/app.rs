use crate::app::event::{AppEvent, AppEventSource};
pub(crate) use crate::app::ui::app_input;
use crate::app::ui::error::ErrorInfo;
use crate::app::ui::file_drop_overlay;
use crate::app::ui::views::UiView;
use crate::app::ui::views::error_view::ErrorView;
use crate::app::ui::views::rom_select_view::RomSelectView;
use crate::app::ui::views::waiting_view::WaitingView;
use crate::emu::commands::EmuCommand;
use crate::emu::events::EmuEvent;
use crate::emu::host::EmuHost;
use crate::shared::frame_buffer::{SharedFrame, SharedFrameHandle};
use anyhow::Context;
use eframe::epaint::TextureHandle;
use egui::Widget;
use nes_core::prelude::Rom;
use std::sync::Arc;

pub enum Action {
    Start,
    Navigate(UiView),
    PlayRom(Vec<u8>),
    AcknowledgeError,
}

pub struct UiCtx<'a> {
    pub frame: &'a SharedFrameHandle,
    pub texture: &'a mut Option<TextureHandle>,
    pub actions: &'a mut Vec<Action>,
}

pub struct App<E: AppEventSource> {
    events: E,
    emu_host: Option<EmuHost>,
    pub(crate) frame: SharedFrameHandle,
    pub(crate) texture: Option<TextureHandle>,
    log_callback: Option<Box<dyn Fn(String) + 'static>>,

    // UI
    view: UiView,
    show_debug: bool,
    user_interacted: bool,
    started: bool,
}

impl<E: AppEventSource> App<E> {
    pub fn new(events: E) -> Self {
        Self {
            events,
            emu_host: None,
            frame: Arc::new(SharedFrame::new()),
            texture: None,

            log_callback: None,
            view: UiView::Waiting(WaitingView::new()),
            show_debug: false,
            user_interacted: false,
            started: false,
        }
    }

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
            Ok((emu)) => {
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

    fn handle_external_events(&mut self) -> anyhow::Result<()> {
        while let Some(event) = self.events.poll_event() {
            self.log("[RECEIVED] handle_external_events()");
            self.handle_external_event(event)?;
        }
        Ok(())
    }

    fn handle_external_event(&mut self, event: AppEvent) -> anyhow::Result<()> {
        match event {
            AppEvent::Start => self.start_emulator(),
            AppEvent::LoadRom(rom) => {
                self.log("AppEvent::LoadRom");
                self.play_rom(rom);
            }
            _ => {
                self.log(format!("\t[Unhandled AppEvent] {:?}", event));
            }
        }
        Ok(())
    }

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

    fn send_command(&self, cmd: EmuCommand) {
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
        self.send_command(EmuCommand::Pause);
        self.view = UiView::Error(ErrorView::new(info));
    }

    fn load_rom_and_start(&mut self, rom_bytes: Vec<u8>) -> anyhow::Result<()> {
        let rom = Rom::parse(&rom_bytes).context("Rom parsing failed")?;
        let cartridge = rom.into_cartridge().context("Cartridge parsing failed")?;
        self.log("Cartridge parsed!");
        self.send_command(EmuCommand::InsertCartridge(cartridge));
        Ok(())
    }

    fn play_rom(&mut self, rom_bytes: Vec<u8>) {
        match self.load_rom_and_start(rom_bytes) {
            Ok(()) => self.view = UiView::playing(),
            Err(e) => self.set_error(e),
        }
    }

    fn apply_actions(&mut self, actions: Vec<Action>) {
        for action in actions {
            self.apply_action(action);
        }
    }

    fn apply_action(&mut self, action: Action) {
        match action {
            Action::Navigate(v) => {
                if let UiView::Error(..) = v {
                    self.send_command(EmuCommand::Pause);
                }
                self.view = v;
            }
            Action::PlayRom(rom_bytes) => {
                self.play_rom(rom_bytes);
            }
            Action::AcknowledgeError => {
                self.view = UiView::RomSelect(RomSelectView::new());
            }
            Action::Start => {
                self.start_emulator();
            }
        }
    }
}

impl<E: AppEventSource> eframe::App for App<E> {
    /// Serves as the main UI loop
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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

        {
            // Build context
            let mut ui_ctx = UiCtx {
                frame: &self.frame,
                texture: &mut self.texture,
                actions: &mut actions,
            };

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
