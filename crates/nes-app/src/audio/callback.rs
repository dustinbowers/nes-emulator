use crate::emu::runtime::EmuRuntime;
use crate::shared::frame_buffer::SharedFrameHandle;
use cpal::{FromSample, Sample, SampleRate, SizedSample};

pub struct AudioCallback {
    runtime: EmuRuntime,
    frame: SharedFrameHandle,
}

impl AudioCallback {
    pub fn new(runtime: EmuRuntime, frame: SharedFrameHandle) -> Self {
        Self { runtime, frame }
    }

    /// Main audio/emulation loop
    pub fn render<T: Sample + SizedSample + FromSample<f32>>(
        &mut self,
        data: &mut [T],
        channels: usize,
        sample_rate: SampleRate,
    ) {
        self.runtime.process_commands();
        self.runtime
            .tick_audio(data, channels, sample_rate, &self.frame);
    }
}
