use cpal::{FromSample, Sample, SampleRate, SizedSample};
use crate::emu::runtime::EmuRuntime;
use crate::shared::frame_buffer::SharedFrameHandle;

pub struct AudioCallback {
    runtime: EmuRuntime,
    frame: SharedFrameHandle
}

impl AudioCallback {
    pub fn new(runtime: EmuRuntime, frame: SharedFrameHandle) -> Self {
        Self { runtime, frame }
    }

    pub fn render<T: Sample + SizedSample + FromSample<f32>>(
        &mut self,
        data: &mut [T],
        channels: usize,
        sample_rate: SampleRate,
    ) {
        // Main audio loop
    }
}