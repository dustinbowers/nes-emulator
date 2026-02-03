use crossbeam_channel::{SendError, Sender};

pub enum AppCommand {
    LoadRom(Vec<u8>),
    Reset,
    Pause(bool),

    Apu(ApuCommand),
    // Debug
    // StepCpu, // TODO
    // StepFrame, // TODO
}

pub enum ApuCommand {
    SetMute { channel: ApuChannel, muted: bool },
}

pub enum ApuChannel {
    Pulse1,
    Pulse2,
    Triangle,
    Noise,
    DMC,
}

#[deprecated]
pub struct AppControl<C> {
    tx: Sender<C>,
}

impl<C> AppControl<C> {
    pub fn new(tx: Sender<C>) -> Self {
        AppControl { tx }
    }

    pub fn send(&self, cmd: C) -> Result<(), SendError<C>> {
        self.tx.send(cmd)
    }
}

impl AppControl<AppCommand> {
    pub fn load_rom(&self, rom: Vec<u8>) {
        let _ = self.tx.send(AppCommand::LoadRom(rom));
    }
    pub fn reset(&self) {
        let _ = self.tx.send(AppCommand::Reset);
    }
    pub fn pause(&self, paused: bool) {
        let _ = self.tx.send(AppCommand::Pause(paused));
    }

    pub fn mute_channel(&self, channel: ApuChannel, muted: bool) {
        let _ = self
            .tx
            .send(AppCommand::Apu(ApuCommand::SetMute { channel, muted }));
    }
}
