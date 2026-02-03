use crossbeam_channel::{Receiver, Sender};
use nes_core::prelude::*;
use crate::emu::commands::EmuCommand;
use crate::emu::events::EmuEvent;

pub struct EmuRuntime {
    nes: NES,
    command_rx: Receiver<EmuCommand>,
    event_tx: Sender<EmuEvent>,
}

impl EmuRuntime {
    pub fn new(command_rx: Receiver<EmuCommand>, event_tx: Sender<EmuEvent>) -> EmuRuntime {
        Self {
            nes: NES::new(),
            command_rx,
            event_tx,
        }
    }
}