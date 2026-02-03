use crate::audio::callback::AudioCallback;
use crate::audio::driver::AudioDriver;
use crate::emu::commands::EmuCommand;
use crate::emu::emu_input::InputState;
use crate::emu::events::EmuEvent;
use crate::emu::runtime::EmuRuntime;
use crate::shared::frame_buffer::{SharedFrame, SharedFrameHandle};
use std::error::Error;
use std::sync::Arc;

/// EmuHost links the UI to the Audio/emulation thread
pub struct EmuHost {
    command_tx: crossbeam_channel::Sender<EmuCommand>,
    event_rx: crossbeam_channel::Receiver<EmuEvent>,
    frame: SharedFrameHandle,

    input_state: InputState,

    // keep-alive
    _stream: cpal::Stream,
}

impl EmuHost {
    pub fn start() -> Result<(Self, SharedFrameHandle), Box<dyn Error>> {
        // Create communication channels
        let (command_tx, command_rx) = crossbeam_channel::unbounded();
        let (event_tx, event_rx) = crossbeam_channel::unbounded();

        // Create shared input state
        let input_state = InputState::new();

        // Create emulator runtime and pass it channels to send/receive messages
        let runtime = EmuRuntime::new(command_rx, event_tx, input_state.clone());

        // Create a new shared frame buffer
        let frame = Arc::new(SharedFrame::new());

        // Create a new audio callback and pass ownership of the new runtime and frame buffer to it
        let audio_callback = AudioCallback::new(runtime, frame.clone());

        // Initialize the audio device
        let mut audio_driver = AudioDriver::init()?;

        // Start a new audio stream with the callback to running the emulator
        let stream = audio_driver.start(audio_callback)?;

        // Tie things together and ship it back to the UI
        let host = Self {
            command_tx,
            event_rx,
            frame: frame.clone(),
            input_state,
            _stream: stream,
        };

        Ok((host, frame))
    }

    pub fn send(&self, cmd: EmuCommand) {
        let _ = self.command_tx.send(cmd);
    }

    pub fn try_recv(&self) -> Option<EmuEvent> {
        self.event_rx.try_recv().ok()
    }

    pub fn set_input(&self, p1: u8, p2: u8) {
        // write to AtomicU8 for sharing input states with runtime
        self.input_state.p1.set(p1);
        self.input_state.p2.set(p2);
    }
}
