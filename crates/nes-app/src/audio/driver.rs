use crate::audio::callback::AudioCallback;
use anyhow::Context;
use cpal::Stream;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub struct AudioDriver {
    device: cpal::Device,
    config: cpal::SupportedStreamConfig,
}

impl AudioDriver {
    pub fn init() -> anyhow::Result<Self> {
        let host = cpal::default_host();

        let device = host
            .default_output_device()
            .context("No output device available")?;

        let config = device
            .default_output_config()
            .context("Failed to get default output config")?;

        Ok(Self { device, config })
    }

    pub fn start(&mut self, mut callback: AudioCallback) -> anyhow::Result<Stream> {
        let sample_rate = self.config.sample_rate();
        let channels = self.config.channels() as usize;
        let config = self.config.clone();

        let stream = self.device.build_output_stream(
            &config.into(),
            move |data: &mut [f32], _| callback.render(data, channels, sample_rate),
            |err| eprintln!("Audio stream error: {}", err),
            None,
        )?;

        // #[cfg(target_arch = "wasm32")]
        stream.play()?;

        Ok(stream)
    }
}
