use crate::app::action::Action;
use crate::app::app::App;

pub trait AppEventSource {
    fn poll_event(&mut self) -> Option<AppEvent>;
}

#[derive(Debug)]
pub enum AppEvent {
    Start,
    LoadRom(Vec<u8>),
    Run,
    Pause,
    Reset,
}

impl<E: AppEventSource> App<E> {
    pub(crate) fn handle_external_events(&mut self) -> anyhow::Result<()> {
        while let Some(event) = self.events.poll_event() {
            self.log("[RECEIVED] handle_external_events()");
            self.handle_external_event(event)?;
        }
        Ok(())
    }

    pub(crate) fn handle_external_event(&mut self, event: AppEvent) -> anyhow::Result<()> {
        match event {
            AppEvent::Start => self.start_emulator(),
            AppEvent::LoadRom(rom) => {
                self.log("AppEvent::LoadRom");
                self.apply_action(Action::PlayRom(rom))
            }
            AppEvent::Run => {
                self.log("AppEvent::Run");
            }
            AppEvent::Pause => {
                self.log("AppEvent::Pause");
                self.apply_action(Action::TogglePause);
            }
            AppEvent::Reset => {
                self.log("AppEvent::Reset");
            }
        }
        Ok(())
    }
}
