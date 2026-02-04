use crate::audio::callback::AudioCallback;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{OutputCallbackInfo, Stream, StreamError};
use std::error::Error;

type DataCallback<T> = Box<dyn FnMut(&mut [T], &OutputCallbackInfo) + Send + 'static>;
type ErrorCallback = Box<dyn FnMut(StreamError) + Send + 'static>;

pub struct AudioDriver {
    host: cpal::Host,
    device: cpal::Device,
    config: cpal::SupportedStreamConfig,
}

impl AudioDriver {
    pub fn init() -> Result<Self, Box<dyn Error>> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("No output device available")?;
        let config = device.default_output_config()?;

        Ok(Self {
            host,
            device,
            config,
        })
    }

    pub fn start(&mut self, mut callback: AudioCallback) -> Result<Stream, Box<dyn Error>> {
        let sample_rate = self.config.sample_rate();
        let channels = self.config.channels() as usize;
        let config = self.config.clone();

        let stream = self.device.build_output_stream(
            &config.into(),
            move |data: &mut [f32], _| callback.render(data, channels, sample_rate),
            |err| eprintln!("Audio stream error: {}", err),
            None,
        )?;

        #[cfg(target_arch = "wasm32")]
        stream.play()?;

        Ok(stream)
    }
}
