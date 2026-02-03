use cpal::SupportedStreamConfig;
use cpal::traits::{DeviceTrait, HostTrait};
use std::error::Error;

// TODO: Include proper options instead of hardcoded defaults?

pub fn init_audio()
-> Result<(impl HostTrait, impl DeviceTrait, SupportedStreamConfig), Box<dyn Error>> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("No output device available")?;
    let config = device.default_output_config()?;

    Ok((host, device, config))
}
