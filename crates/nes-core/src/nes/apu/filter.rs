use std::f32::consts::PI;

#[deprecated]
#[derive(Clone, Copy)]
pub struct OnePole {
    prev_in: f32,
    prev_out: f32,
}

impl Default for OnePole {
    fn default() -> Self {
        Self::new()
    }
}

impl OnePole {
    pub fn new() -> Self {
        Self {
            prev_in: 0.0,
            prev_out: 0.0,
        }
    }

    /// High-pass filter
    pub fn high_pass(&mut self, input: f32, cutoff_hz: f32, sample_rate: f32) -> f32 {
        let rc = 1.0 / (2.0 * PI * cutoff_hz);
        let dt = 1.0 / sample_rate;
        let alpha = rc / (rc + dt);

        let output = alpha * (self.prev_out + input - self.prev_in);

        self.prev_in = input;
        self.prev_out = output;

        output
    }

    /// Low-pass filter
    pub fn low_pass(&mut self, input: f32, cutoff_hz: f32, sample_rate: f32) -> f32 {
        let rc = 1.0 / (2.0 * PI * cutoff_hz);
        let dt = 1.0 / sample_rate;
        let alpha = dt / (rc + dt);

        let output = self.prev_out + alpha * (input - self.prev_out);
        self.prev_out = output;

        output
    }
}
