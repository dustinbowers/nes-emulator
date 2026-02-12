const ENV_LOOP: u8 = 0b0010_0000;
const ENV_CONST: u8 = 0b0001_0000;
const ENV_VOLUME: u8 = 0b0000_1111;

#[inline]
fn env_volume(v: u8) -> u8 {
    v & ENV_VOLUME
}
#[inline]
fn env_loop(v: u8) -> bool {
    (v & ENV_LOOP) != 0
}
#[inline]
fn env_const(v: u8) -> bool {
    (v & ENV_CONST) != 0
}

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

    #[cfg(test)]
    fn start(&mut self) {
        self.start = false;
        self.decay = 15;
        self.divider = self.period;
    }

    /// Called with value in this shape: --LC_VVVV
    pub fn set(&mut self, value: u8) {
        let v = env_volume(value);
        self.period = v;
        self.constant_volume = v;

        self.volume_mode = if env_const(value) {
            VolumeMode::Constant
        } else {
            VolumeMode::Envelope
        };

        self.loop_flag = env_loop(value);
    }

    /// Called by the quarter-frame clock
    pub fn clock(&mut self) {
        // If start is set, just reload and wait for next clock to advance
        if self.start {
            self.start = false;
            self.decay = 15;
            self.divider = self.period;
            return;
        }

        if self.divider == 0 {
            self.divider = self.period; // reload divider
            if self.decay > 0 {
                self.decay -= 1;
            } else if self.loop_flag {
                // if decay == 0 && loop enabled
                self.decay = 15;
            }
        } else {
            self.divider -= 1;
        }
    }

    pub fn set_start_flag(&mut self, start: bool) {
        self.start = start;
    }

    pub fn set_volume(&mut self, volume: u8) {
        let v = env_volume(volume);
        self.period = v;
        self.constant_volume = v;
    }

    pub fn output(&self) -> u8 {
        match self.volume_mode {
            VolumeMode::Envelope => self.decay,
            VolumeMode::Constant => self.constant_volume,
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

    pub fn get_start_flag(&self) -> bool {
        self.start
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(env: &mut Envelope, n: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            out.push(env.output());
            env.clock();
        }
        out
    }

    #[test]
    fn start_is_consumed_on_next_clock_and_resets_decay_and_divider() {
        let mut env = Envelope::new();
        env.set(0b0000_0100); // period = 4
        env.decay = 7;
        env.divider = 1;

        env.set_start_flag(true);

        // before clock, nothing forced yet
        assert_eq!(env.get_start_flag(), true);

        env.clock();

        assert_eq!(env.get_start_flag(), false);
        assert_eq!(env.decay, 15);
        assert_eq!(env.divider, 4);
    }

    #[test]
    fn start_clock_does_not_also_step_decay() {
        let mut env = Envelope::new();
        env.set(0b0000_0000); // period = 0 (would step every clock)
        env.set_start_flag(true);

        // first clock consumes start and loads decay=15
        env.clock();
        assert_eq!(env.decay, 15);

        // second clock (period=0) should decrement decay to 14
        env.clock();
        assert_eq!(env.decay, 14);
    }

    #[test]
    fn envelope_decrements_every_period_plus_one_clocks() {
        let mut env = Envelope::new();
        env.set(0b0000_0011); // period = 3
        env.set_start_flag(true);
        env.clock(); // consume start

        // decay=15 and period=3, decrement happens every 4 clocks
        // 3->2->1->0 (no change), next clock reloads divider and decrements decay
        let outs = step(&mut env, 10);

        // outs[0] is after start consumed, before any further clocks
        assert_eq!(outs[0], 15); // 3
        assert_eq!(outs[1], 15); // 2
        assert_eq!(outs[2], 15); // 1
        assert_eq!(outs[3], 15); // 0
        assert_eq!(outs[4], 14); // first decrement
    }

    #[test]
    fn period_zero_decrements_every_clock_after_start_consumed() {
        let mut env = Envelope::new();
        env.set(0b0000_0000);
        env.set_start_flag(true);
        env.clock(); // consume start => decay=15

        let outs = step(&mut env, 6);
        assert_eq!(outs, vec![15, 14, 13, 12, 11, 10]);
    }

    #[test]
    fn loop_flag_reloads_decay_from_zero_to_15() {
        let mut env = Envelope::new();
        env.set(0b0010_0000 | 0b0000_0001); // loop=1, period=1
        env.set_start_flag(true);
        env.clock(); // consume start => 15

        // period=1 => decrement every 2 clocks
        // need enough clocks to hit 0 then wrap
        let outs = step(&mut env, 80);

        assert!(outs.contains(&0), "never reached 0");
        // after reaching 0, it should eventually go back to 15
        let i0 = outs.iter().position(|&v| v == 0).unwrap();
        assert!(
            outs[i0..].contains(&15),
            "never looped back to 15 after reaching 0"
        );
    }

    #[test]
    fn constant_volume_output_ignores_decay() {
        let mut env = Envelope::new();
        env.set(0b0001_1010); // constant mode, volume=10
        env.set_start_flag(true);
        env.clock(); // consume start

        let outs = step(&mut env, 20);
        assert!(outs.iter().all(|&v| v == 10));
    }

    #[test]
    fn set_volume_masks_to_4_bits_and_updates_constant_volume() {
        let mut env = Envelope::new();
        env.set(0b0001_0000); // constant mode
        env.set_volume(0xFE); // should become 14

        assert_eq!(env.output(), 14);
        assert_eq!(env.get_divider_period(), 14);
    }
}
