pub struct Envelope {
    start: bool,
    divider: u8,
    decay: u8,
    constant_volume: u8,
    loop_flag: bool,
    volume_mode: VolumeMode,
    period: u8,
}

#[derive(Debug, PartialEq, Clone)]
pub enum VolumeMode {
    Envelope,
    Constant,
}

impl Envelope {
    pub fn new() -> Envelope {
        Envelope {
            start: false,
            divider: 0,
            decay: 0,
            constant_volume: 0,
            loop_flag: false,
            volume_mode: VolumeMode::Envelope,
            period: 0,
        }
    }

    pub fn start(&mut self) {
        self.start = false;
        self.decay = 15;
        self.divider = self.period;
    }

    pub fn set(&mut self, value: u8) {
        self.period = value & 0b0000_1111; // lower 4 bits
        self.constant_volume = self.period;
        self.loop_flag = (value & 0b0010_0000) != 0; // bit 5
        self.volume_mode = if (value & 0b0001_0000) == 0 {
            // bit 4
            VolumeMode::Envelope
        } else {
            VolumeMode::Constant
        };
    }

    /// Called by the quarter-frame clock
    pub fn clock(&mut self) {
        if self.start {
            self.start = false;
            self.decay = 15;
            self.divider = self.period;
        } else if self.divider == 0 {
            self.divider = self.period;
            if self.decay > 0 {
                self.decay -= 1;
            } else if self.loop_flag {
                self.decay = 15;
            }
        } else {
            self.divider -= 1;
        }
    }

    pub fn output(&self) -> u8 {
        if self.volume_mode == VolumeMode::Constant {
            self.constant_volume
        } else {
            self.decay
        }
    }

    pub fn get_volume_mode(&self) -> VolumeMode {
        self.volume_mode.clone()
    }
    pub fn get_divider_period(&self) -> u8 {
        self.period
    }
    pub fn get_loop_flag(&self) -> bool {
        self.loop_flag
    }
    pub fn set_start_flag(&mut self, start: bool) {
        self.start = start;
    }
    pub fn get_start_flag(&self) -> bool {
        self.start
    }
    pub fn set_volume(&mut self, volume: u8) {
        self.period = volume;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to clock envelope multiple times and collect outputs
    fn clock_envelope(env: &mut Envelope, clocks: usize) -> Vec<u8> {
        let mut outputs = Vec::with_capacity(clocks);
        for _ in 0..clocks {
            outputs.push(env.output());
            env.clock();
        }
        outputs
    }

    #[test]
    fn test_envelope_start() {
        let mut env = Envelope::new();
        env.set(0b0001_1111); // period = 15, disable = true
        env.start();

        // When started, decay resets to 15 and divider to period
        assert_eq!(env.decay, 15);
        assert_eq!(env.divider, env.period);
        assert!(!env.start);
    }

    #[test]
    fn test_envelope_clock_countdown() {
        let mut env = Envelope::new();
        env.set(0b0000_0100); // period = 4
        env.start();

        // Each decay step takes period + 1 = 5 clocks
        let outputs = clock_envelope(&mut env, 25);

        // Check first few decay steps
        assert_eq!(outputs[0], 15); // immediately after start
        assert_eq!(outputs[4], 15); // last clock with decay = 15
        assert_eq!(outputs[5], 14); // first clock with decay = 14
        assert_eq!(outputs[9], 14); // last clock with decay = 14
        assert_eq!(outputs[10], 13); // first clock with decay = 13
    }

    #[test]
    fn test_envelope_loop_flag() {
        let mut env = Envelope::new();
        env.set(0b0010_0011); // period = 3, loop_flag = true, disable = false
        env.start();

        // Each decay step takes period + 1 = 4 clocks, 16 decay steps = 64 clocks
        let outputs = clock_envelope(&mut env, 70);

        // Check that after hitting 0, it loops back to 15
        assert!(outputs.contains(&0));
        assert!(outputs.contains(&15));

        // Ensure it actually loops multiple times
        let first_zero_index = outputs.iter().position(|&x| x == 0).unwrap();
        let next_fifteen_index = outputs
            .iter()
            .skip(first_zero_index)
            .position(|&x| x == 15)
            .unwrap();
        assert!(next_fifteen_index > 0);
    }

    #[test]
    fn test_envelope_constant_volume() {
        let mut env = Envelope::new();
        env.set(0b0001_1010); // period = 10, disable = true
        env.start();

        let outputs = clock_envelope(&mut env, 20);

        // When disable is true, output is constant_volume, not decay
        for &o in &outputs {
            assert_eq!(o, env.constant_volume);
        }
    }

    #[test]
    fn test_envelope_disable_false_decay() {
        let mut env = Envelope::new();
        env.set(0b0000_0101); // period = 5, disable = false
        env.start();

        let outputs = clock_envelope(&mut env, 40);

        // Decay should decrement at the correct times
        let mut last_decay = outputs[0];
        for &curr_decay in &outputs[1..] {
            if curr_decay != last_decay {
                // Decay should decrease by 1 each (period + 1) clocks
                assert!(curr_decay < last_decay);
            }
            last_decay = curr_decay;
        }
    }

    #[test]
    fn test_envelope_looping_multiple_times() {
        let mut env = Envelope::new();
        env.set(0b0010_0100); // period = 4, loop_flag = true, disable = false
        env.start();

        // Clock enough to wrap at least twice
        let outputs = clock_envelope(&mut env, 150);

        // Find first zero
        let first_zero_index = outputs.iter().position(|&v| v == 0).unwrap();
        // Find next 15 after first zero
        let next_fifteen_index = outputs
            .iter()
            .enumerate()
            .skip(first_zero_index)
            .find(|&(_, &v)| v == 15)
            .unwrap()
            .0;

        assert!(
            first_zero_index < next_fifteen_index,
            "Envelope did not loop back to 15 after hitting 0"
        );
    }
}
