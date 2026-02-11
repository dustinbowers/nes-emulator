use crate::emu::commands::{AudioChannel, EmuCommand};
use crate::emu::emu_input::InputState;
use crate::emu::event::EmuEvent;
use crate::shared::frame_buffer::SharedFrameHandle;
use cpal::{FromSample, Sample, SampleRate, SizedSample};
use crossbeam_channel::{Receiver, Sender};
use nes_core::prelude::*;

const PPU_HZ: u64 = 5_369_318;

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

    /// Handle EmuCommands received from the App UI thread
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
                EmuCommand::ToggleAudioChannel(audio_channel) => match audio_channel {
                    AudioChannel::Pulse1 => self.nes.bus.apu.mute_pulse1 ^= true,
                    AudioChannel::Pulse2 => self.nes.bus.apu.mute_pulse2 ^= true,
                    AudioChannel::Triangle => self.nes.bus.apu.mute_triangle ^= true,
                    AudioChannel::Noise => self.nes.bus.apu.mute_noise ^= true,
                    AudioChannel::DMC => self.nes.bus.apu.mute_dmc ^= true,
                },
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

        let sr_u64 = sample_rate as u64;
        self.nes.bus.apu.set_sample_rate(sample_rate as f64);

        let base_ticks = PPU_HZ / sr_u64;
        let frac = PPU_HZ % sr_u64; // Bresenham remainder

        for frame in data.chunks_mut(channels) {
            // Update user input
            self.nes.bus.joypads[0].set_buttons(self.input_state.p1.load());
            self.nes.bus.joypads[1].set_buttons(self.input_state.p2.load());

            // Determine how many PPU ticks to run this sample
            let mut ticks = base_ticks;
            self.nes.ppu_remainder += frac;
            if self.nes.ppu_remainder >= sr_u64 {
                self.nes.ppu_remainder -= sr_u64;
                ticks += 1;
            }

            // deterministically sample with 2-tap average
            let start = self.nes.bus.apu.get_last_sample();
            for _ in 0..ticks {
                if self.nes.tick() {
                    frame_buffer.write(self.nes.get_frame_buffer());
                }
            }

            let end = self.nes.bus.apu.get_last_sample();
            let raw = 0.5 * (start + end);

            // run raw sample through filters
            let out_f32 = self.nes.bus.apu.filter_raw_sample(raw);

            let sample = T::from_sample(out_f32);
            for out in frame.iter_mut() {
                *out = sample;
            }
        }
    }
}
