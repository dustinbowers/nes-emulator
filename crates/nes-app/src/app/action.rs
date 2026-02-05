use crate::app::app::App;
use crate::app::event::AppEventSource;
use crate::app::ui::views::UiView;
use crate::app::ui::views::rom_select_view::RomSelectView;
use crate::emu::commands::EmuCommand;

pub enum Action {
    Start,
    Navigate(UiView),
    PlayRom(Vec<u8>),
    AcknowledgeError,
    TogglePause,
    SetPaused(bool),
}

impl<E: AppEventSource> App<E> {
    pub(crate) fn apply_actions(&mut self, actions: Vec<Action>) {
        for action in actions {
            self.apply_action(action);
        }
    }

    pub(crate) fn apply_action(&mut self, action: Action) {
        match action {
            Action::Navigate(v) => {
                if let UiView::Error(..) = v {
                    self.send_command(EmuCommand::Pause(true));
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
            Action::TogglePause => {
                self.paused = !self.paused;
                self.apply_action(Action::SetPaused(self.paused));
            }
            Action::SetPaused(p) => {
                self.log("[Apply Action::Pause]");
                self.paused = p;
                self.send_command(EmuCommand::Pause(self.paused));
            }
        }
    }
}
