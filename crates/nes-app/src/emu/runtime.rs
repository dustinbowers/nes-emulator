use crate::emu::commands::EmuCommand;
use crate::emu::emu_input::InputState;
use crate::emu::event::EmuEvent;
use crate::shared::frame_buffer::SharedFrameHandle;
use cpal::{FromSample, Sample, SampleRate, SizedSample};
use crossbeam_channel::{Receiver, Sender};
use nes_core::prelude::*;

pub struct EmuRuntime {
    nes: NES,
    input_state: InputState,
    command_rx: Receiver<EmuCommand>,
    event_tx: Sender<EmuEvent>,
    paused: bool,
}

impl EmuRuntime {
    pub fn new(
        command_rx: Receiver<EmuCommand>,
        event_tx: Sender<EmuEvent>,
        input_state: InputState,
    ) -> EmuRuntime {
        Self {
            nes: NES::new(),
            input_state,
            command_rx,
            event_tx,
            paused: false,
        }
    }

    pub fn process_commands(&mut self) {
        while let Ok(command) = self.command_rx.try_recv() {
            match command {
                EmuCommand::InsertCartridge(cartridge) => {
                    self.event_tx
                        .send(EmuEvent::Log("[Audio thread] InsertCartridge!".into()))
                        .ok();
                    self.nes.insert_cartridge(cartridge);
                    self.paused = false;
                }
                EmuCommand::Reset => {
                    self.nes.bus.reset_components();
                    self.paused = true;
                }
                EmuCommand::Pause(p) => {
                    self.paused = p;
                }
            }
        }
    }

    pub fn tick_audio<T>(
        &mut self,
        data: &mut [T],
        channels: usize,
        sample_rate: SampleRate,
        frame_buffer: &SharedFrameHandle,
    ) where
        T: Sample + SizedSample + FromSample<f32>,
    {
        if self.paused {
            for audio_frame in data.chunks_mut(channels) {
                for out in audio_frame.iter_mut() {
                    *out = T::from_sample(0.0);
                }
            }
            return;
        }

        let sample_rate = sample_rate as f64;
        // PPU cycles per audio sample (5.369318 MHz / 44.1 kHz)
        let mut frame_ready = false;
        let ppu_cycles_per_sample = 5369318.0 / sample_rate;
        let mut cycle_acc = self.nes.cycle_acc;

        for audio_frame in data.chunks_mut(channels) {
            cycle_acc += ppu_cycles_per_sample;

            // Update user input
            self.nes.bus.joypads[0].set_buttons(self.input_state.p1.load());
            self.nes.bus.joypads[1].set_buttons(self.input_state.p2.load());

            while cycle_acc >= 1.0 {
                if self.nes.tick() {
                    frame_ready = true;
                    frame_buffer.write(self.nes.get_frame_buffer());
                }
                cycle_acc -= 1.0;
            }

            let raw = self.nes.bus.apu.sample();
            let sample = T::from_sample(raw);

            for out in audio_frame.iter_mut() {
                *out = sample;
            }
        }
        self.nes.cycle_acc = cycle_acc;
    }
}
