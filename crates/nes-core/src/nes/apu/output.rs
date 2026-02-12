pub struct ApuOutput {
    blip: blip_buf::BlipBuf,
    cpu_hz: f64,
    sample_rate: u32,
    t_cpu: u32,

    scratch_i16: Vec<i16>,
}

impl ApuOutput {
    pub fn new(cpu_hz: f64, sample_rate: u32, max_samples: usize) -> Self {
        let mut blip = blip_buf::BlipBuf::new(max_samples as u32);
        blip.set_rates(cpu_hz, sample_rate as f64);

        Self {
            blip,
            cpu_hz,
            sample_rate,
            t_cpu: 0,
            scratch_i16: vec![0; max_samples],
        }
    }

    pub fn reset(&mut self) {
        self.blip.clear();
        self.t_cpu = 0;
    }

    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        if self.sample_rate == sample_rate {
            return;
        }
        self.sample_rate = sample_rate;
        self.blip.set_rates(self.cpu_hz, sample_rate as f64);

        // Clear when sample rate changes
        self.blip.clear();
        self.t_cpu = 0;
    }

    #[inline]
    pub fn add_delta(&mut self, delta: i32) {
        if delta != 0 {
            self.blip.add_delta(self.t_cpu, delta);
        }
    }

    #[inline]
    pub fn step_cpu_cycle(&mut self) {
        self.t_cpu += 1;
    }

    pub fn end_frame(&mut self) {
        let clocks = self.t_cpu;
        self.blip.end_frame(clocks);
        self.t_cpu = 0;
    }

    pub fn samples_available(&self) -> usize {
        self.blip.samples_avail() as usize
    }

    pub fn clocks_needed(&self, sample_count: u32) -> u32 {
        self.blip.clocks_needed(sample_count)
    }

    /// Returns how many samples were actually written
    pub fn read_samples_f32(&mut self, out: &mut [f32]) -> usize {
        let want = out.len();

        // ensure scratch big enough
        if self.scratch_i16.len() < want {
            self.scratch_i16.resize(want, 0);
        }

        let got = self.blip.read_samples(&mut self.scratch_i16[..want], false);

        // Scale to [-1.0, 1.0)
        for i in 0..got {
            out[i] = (self.scratch_i16[i] as f32) / 32768.0;
        }

        got
    }
}
