use crate::app::event::{AppEvent, AppEventSource};
use crate::app::ui::{file_drop_overlay, Transition};
use crate::app::ui::views::UiView;
pub(crate) use crate::app::ui::app_input;
use crate::emu::commands::EmuCommand;
use crate::emu::events::EmuEvent;
use crate::emu::host::EmuHost;
use crate::shared::frame_buffer::SharedFrameHandle;
use anyhow::Context;
use eframe::epaint::TextureHandle;
use nes_core::prelude::Rom;
use crate::app::ui::views::waiting_view::WaitingView;

pub enum Action {
    PlayRom(Vec<u8>)
}

pub struct UiCtx<'a> {
    pub frame: &'a Option<SharedFrameHandle>,
    pub texture: &'a mut Option<TextureHandle>,
    pub actions: &'a mut Vec<Action>,
}

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

impl<E: AppEventSource> App<E> {
    pub fn new(events: E) -> Self {
        Self {
            events,
            emu_host: None,
            frame: None,
            texture: None,

            log_callback: None,
            view: UiView::Waiting(WaitingView::new()),
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

    fn handle_external_event(&mut self, event: AppEvent) -> anyhow::Result<()> {
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
            Transition::Switch(v) => {
                if let UiView::Error(err) = &v {
                    self.log(format!("UI error - {}: {}", err.context, err.details));
                }
                self.view = v;
            }
            Transition::Quit => { /* TODO */ }
        }
    }

    // fn ui_ctx(&mut self) -> UiCtx<'_> {
    //     UiCtx {
    //         frame: &self.frame,
    //         texture: &mut self.texture,
    //         actions: &mut vec![],
    //     }
    // }
}

impl<E: AppEventSource> eframe::App for App<E> {
    /// Serves as the main UI loop
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Err(e) = self.handle_external_events() {
            self.set_error(e.to_string());
        }
        self.handle_emu_events();

        // Update controller states while playing
        if matches!(self.view, UiView::Playing { .. })
            && let Some(emu_host) = self.emu_host.as_ref()
        {
            app_input::update_controller_state(ctx, &emu_host);
        }

        let mut pending_rom: Option<Vec<u8>> = None;

        // Render view
        let mut transition = {
            // Build context
            let mut ui_ctx = UiCtx {
                frame: &self.frame,
                texture: &mut self.texture,
                actions: &mut vec![],
            };
            match &mut self.view {
                UiView::Waiting(view) => {
                    view.ui::<E>(ctx);
                    pending_rom  = view.rom_bytes.take();
                    Transition::None
                }
                UiView::Options => Transition::None,
                UiView::Playing(view) => view.ui::<E>(ctx, &mut ui_ctx),
                UiView::Error(_) => Transition::None,
            }
        };

        file_drop_overlay::handle_file_drop(ctx, |bytes| {
            self.log(format!("File drop detected! ({} bytes)", bytes.len()));
            let t = self.play_rom(bytes);
            self.apply_transition(t);
        });

        if let Some(rom_bytes) = pending_rom {
           transition = self.play_rom(rom_bytes);
        }

        // Handle transitions
        self.apply_transition(transition);

        ctx.request_repaint();
    }
}
