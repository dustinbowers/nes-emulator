use crossbeam_channel::{Receiver, SendError, Sender, TryRecvError};

pub enum AppCommand {
    LoadRom(Vec<u8>),
    Reset,
    Pause(bool),
}

pub struct AppControl<C> {
    tx: Sender<C>,
    rx: Receiver<C>,
}

impl<C> AppControl<C> {
    pub fn new(tx: Sender<C>, rx: Receiver<C>) -> Self {
        AppControl { tx, rx }
    }

    pub fn send(&self, cmd: C) -> Result<(), SendError<C>> {
        self.tx.send(cmd)
    }

    pub fn receive(&self) -> Result<C, TryRecvError> {
        self.rx.try_recv()
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
}
