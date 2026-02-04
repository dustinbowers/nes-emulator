use anyhow::Context;
use crate::app::event::{AppEvent, AppEventSource};
use crate::app::ui::{app_input};
use crate::emu::commands::EmuCommand;
use crate::emu::host::EmuHost;
use crate::shared::frame_buffer::SharedFrameHandle;
use eframe::epaint::TextureHandle;
use nes_core::prelude::{Rom};
use crate::app::ui::views::playing_view::PlayingView;
use crate::emu::events::EmuEvent;

pub struct App<E: AppEventSource> {
    events: E,
    emu_host: Option<EmuHost>,
    pub(crate) frame: Option<SharedFrameHandle>,
    pub(crate) texture: Option<TextureHandle>,
    log_callback: Option<Box<dyn Fn(String) + 'static>>,

    // UI
    view: UiView,
    user_interacted: bool,
    show_debug: bool,
    last_error: Option<String>,
}

pub enum Transition {
    None,
    Switch(UiView),
    Quit,
}

impl Transition {
    fn to(view: UiView) -> Self { Transition::Switch(view) }
}


pub struct ErrorInfo {
    context: String,
    details: String,
}

impl ErrorInfo {
    pub fn from_anyhow(context: impl Into<String>, err: anyhow::Error) -> Self {
        let context = context.into();
        let details = format!("{err:#}");
        Self { context, details }
    }
}


enum UiView {
    Waiting,
    Options,
    Playing(PlayingView),
    Error(ErrorInfo)
}

impl UiView {
    fn playing() -> Self { UiView::Playing(PlayingView::new()) }

    fn error_load_rom(err: anyhow::Error) -> Self {
        UiView::Error(ErrorInfo::from_anyhow("Failed to load ROM", err))
    }
}


pub struct UiCtx<'a> {
    pub frame: &'a Option<SharedFrameHandle>,
    pub texture: &'a mut Option<TextureHandle>,
}

impl<E: AppEventSource> App<E> {
    pub fn new(events: E) -> Self {
        Self {
            events,
            emu_host: None,
            frame: None,
            texture: None,

            log_callback: None,
            view: UiView::Waiting,
            user_interacted: false,
            show_debug: false,
            last_error: None,
        }
    }

    pub fn start(&mut self) {
        if self.emu_host.is_some() {
            panic!("Double App::start() shouldn't happen");
        }
        self.log("App::start()");
        match EmuHost::start() {
            Ok((emu, frame)) => {
                self.log("EmuHost::start() => Ok()");
                self.emu_host = Some(emu);
                self.frame = Some(frame);
            }
            Err(e) => {
                self.log(format!("EmuHost::start() => Err(): {}", e));
                self.last_error = Some(e.to_string());
            }
        }
    }

    pub fn with_initial_events(mut self, events: impl IntoIterator<Item = AppEvent>) -> Self {
        for event in events {
            if let Err(e) = self.handle_external_event(event) {
                self.last_error = Some(e.to_string());
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

    fn handle_external_event(&mut self, event: AppEvent) -> anyhow::Result<()>{
        match event {
            AppEvent::Start => self.start(),
            AppEvent::LoadRom(rom) => {
                self.log("AppEvent::LoadRom");
                let t = self.play_rom(rom);
                self.apply_transition(t);
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

    fn set_error(&mut self, error: String) {
        self.last_error = Some(error);
    }

    fn load_rom_and_start(&mut self, rom_bytes: Vec<u8>) -> anyhow::Result<()> {
        let rom = Rom::parse(&rom_bytes).context("Rom parsing failed")?;
        let cartridge = rom.into_cartridge().context("Cartridge parsing failed")?;
        self.log("Cartridge parsed!");
        self.send_command(EmuCommand::InsertCartridge(cartridge));
        Ok(())
    }

    fn play_rom(&mut self, rom_bytes: Vec<u8>) -> Transition {
        match self.load_rom_and_start(rom_bytes) {
            Ok(()) => Transition::to(UiView::playing()),
            Err(e) => Transition::to(UiView::error_load_rom(e)),
        }
    }

    fn apply_transition(&mut self, t: Transition) {
        match t {
            Transition::None => {}
            Transition::Switch(v) => { self.view = v; }
            Transition::Quit => { /* TODO */ }
        }
    }

    fn ui_ctx(&mut self) -> UiCtx<'_> {
        UiCtx {
            frame: &self.frame,
            texture: &mut self.texture,
        }
    }
}

impl<E: AppEventSource> eframe::App for App<E> {
    /// Serves as the main UI loop
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Err(e) = self.handle_external_events() {
            self.set_error(e.to_string());
        }
        self.handle_emu_events();

        // if let (Some(emu_host), Some(frame)) = (self.emu_host.as_ref(), self.frame.as_ref()) {
        //     app_input::update_controller_state(ctx, &emu_host);
        //
        //     main_view::render(ctx, &mut self.texture, &frame, false);
        //
        //     file_drop_overlay::handle_file_drop(ctx, |bytes| {
        //         self.log(format!("File drop detected! ({} bytes)", bytes.len()));
        //         if let Err(e) = self.play_rom(bytes) {
        //             self.set_error(e.to_string());
        //         }
        //     });
        //
        //     if self.show_debug {
        //         // debug ui
        //     }
        //
        // } else {
        //     if let Some(rom_data) = waiting_view::render(ctx) {
        //         self.play_rom(rom_data).ok();
        //     }
        // }

        // Update controller states while playing
        if matches!(self.view, UiView::Playing{..}) && let Some(emu_host) = self.emu_host.as_ref() {
            app_input::update_controller_state(ctx, &emu_host);
        }

        // Render view
        let transition = {
            // Build context
            let mut ui_ctx = UiCtx {
                frame: &self.frame,
                texture: &mut self.texture,
            };
            match &mut self.view {
                UiView::Waiting => Transition::None,
                UiView::Options => Transition::None,
                UiView::Playing(view) => {
                    view.ui::<E>(ctx, &mut ui_ctx)
                },
                UiView::Error(_) => Transition::None,
            }
        };

        // Handle transitions
        self.apply_transition(transition);

        ctx.request_repaint();
    }
}
