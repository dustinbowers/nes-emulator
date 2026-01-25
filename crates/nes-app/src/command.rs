use crossbeam_channel::{SendError, Sender};

pub enum AppCommand {
    LoadRom(Vec<u8>),
    Reset,
    Pause(bool),

    // Debug / control
    SetApuMute { channel: ApuChannel, muted: bool },
    // StepCpu, // TODO
    // StepFrame, // TODO
}

pub enum ApuChannel {
    Pulse1,
    Pulse2,
    Triangle,
    Noise,
    DMC,
}

pub struct AppControl<C> {
    tx: Sender<C>,
    // rx: Receiver<C>,
}

impl<C> AppControl<C> {
    pub fn new(tx: Sender<C>) -> Self {
        AppControl { tx }
    }

    pub fn send(&self, cmd: C) -> Result<(), SendError<C>> {
        self.tx.send(cmd)
    }

    // pub fn receive(&self) -> Result<C, TryRecvError> {
    //     self.rx.try_recv()
    // }
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
}
