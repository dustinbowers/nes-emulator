use crate::apu::SAMPLE_RATE;

pub struct SquareWave {
    phase: f32,
    freq: f64,
}

impl SquareWave {
    pub fn new(freq: f64) -> SquareWave {
        SquareWave { phase: 0.0, freq }
    }
    pub fn sample(&mut self) -> i16 {
        let freq = 440.0;
        let sample = (self.phase.sin() * i16::MAX as f32) as i16;
        self.phase += 2.0 * std::f32::consts::PI * freq / SAMPLE_RATE;
        sample
    }
}
