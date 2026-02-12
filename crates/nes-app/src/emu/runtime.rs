use crate::emu::commands::{AudioChannel, EmuCommand};
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

    scratch_buf: Vec<f32>,
    last_sample_rate: Option<u32>,
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
            scratch_buf: Vec::new(),
            last_sample_rate: None,
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

    fn run_cpu_cycles(&mut self, cpu_cycles: u32, frame_buffer: &SharedFrameHandle) {
        let mut ran = 0;
        while ran < cpu_cycles {
            // Run the PPU until we hit a CPU tick
            loop {
                let (cpu_tick, frame_ready) = self.nes.tick();
                if frame_ready {
                    frame_buffer.write(self.nes.get_frame_buffer());
                }

                if cpu_tick {
                    break;
                }
            }
            ran += 1;
        }
        self.nes.bus.apu.end_frame();
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
            for out in data.iter_mut() {
                *out = T::from_sample(0.0);
            }
            return;
        }

        let frames = data.len() / channels;
        // Only update APU SR when it changes
        if self.last_sample_rate != Some(sample_rate) {
            self.nes.bus.apu.set_sample_rate(sample_rate as f64);
            self.last_sample_rate = Some(sample_rate);
        }

        if self.scratch_buf.len() < frames {
            self.scratch_buf.resize(frames, 0.0);
        }

        // update user input
        self.nes.bus.joypads[0].set_buttons(self.input_state.p1.load());
        self.nes.bus.joypads[1].set_buttons(self.input_state.p2.load());

        // Let the emulator run ahead a little bit to provide a
        // buffer in case the audio thread gets bogged down
        let target_available = frames * 3;

        // run cpu until blip buffer has enough frames
        while self.nes.bus.apu.samples_available() < target_available {
            let samples_available = self.nes.bus.apu.samples_available() as u32;
            let need_samples = target_available as u32 - samples_available;

            let mut cpu_cycles = self.nes.bus.apu.clocks_needed(need_samples);

            // avoid stalling if clocks_needed() is zero
            if cpu_cycles == 0 {
                cpu_cycles = 1;
            }

            self.run_cpu_cycles(cpu_cycles, frame_buffer);
        }

        let got = self
            .nes
            .bus
            .apu
            .read_samples_f32(&mut self.scratch_buf[..frames]);

        // If for some reason we get less than needed, fill remaining with silence
        if got < frames {
            for s in &mut self.scratch_buf[got..frames] {
                *s = 0.0;
            }
        }

        // fill output channels
        for (i, frame) in data.chunks_mut(channels).enumerate() {
            let s = T::from_sample(self.scratch_buf[i]);
            for out in frame.iter_mut() {
                *out = s;
            }
        }
    }
}
