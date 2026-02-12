pub enum PulseType {
    Pulse1,
    Pulse2,
}

pub struct Sweep {
    pulse_type: PulseType,
    enabled: bool,
    reload: bool,
    negate: bool,

    period: u8,  // 0-7
    shift: u8,   // 0-7
    divider: u8, // counts down
}

impl Sweep {
    pub fn new(pulse_type: PulseType) -> Sweep {
        Sweep {
            pulse_type,
            enabled: false,
            reload: false,
            negate: false,

            period: 0,
            shift: 0,
            divider: 0,
        }
    }

    pub fn set(&mut self, value: u8) {
        // value: 0bEPPP_NSSS
        self.enabled = value & 0b1000_0000 != 0;
        self.period = (value >> 4) & 0b111;
        self.negate = value & 0b0000_1000 != 0;
        self.shift = value & 0b0000_0111;
        self.reload = true;
    }

    pub fn compute_target(&self, timer: u16) -> u16 {
        let change = timer >> self.shift;
        if self.negate {
            match self.pulse_type {
                PulseType::Pulse1 => timer.wrapping_sub(change).wrapping_sub(1),
                PulseType::Pulse2 => timer.wrapping_sub(change),
            }
        } else {
            timer.wrapping_add(change)
        }
    }

    /// Called on half-frame clocks
    pub fn clock(&mut self, timer: &mut u16) {
        // If reload, just reload and wait for next clock before ticking
        if self.reload {
            self.divider = self.period;
            self.reload = false;
            return;
        }

        // tick the divider
        if self.divider == 0 {
            // When divider hits 0, sweep only if enabled and shift > 0
            if self.enabled && self.shift > 0 {
                let target = self.compute_target(*timer);

                // skip out of range frequencies
                if (8..=0x7FF).contains(&target) {
                    *timer = target;
                }
            }
            // reload divider
            self.divider = self.period;
        } else {
            // otherwise decrement divider
            self.divider -= 1;
        }
    }

    pub fn is_muting(&self, timer: u16) -> bool {
        if timer < 8 {
            return true;
        }
        if !self.enabled || self.shift == 0 {
            return false;
        }
        let target = self.compute_target(timer);
        target > 0x7FF
    }

    #[cfg(test)]
    fn divider(&self) -> u8 {
        self.divider
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to quickly set sweep state
    fn sweep_cfg(enabled: bool, period: u8, negate: bool, shift: u8) -> u8 {
        let e = if enabled { 1 } else { 0 };
        let n = if negate { 1 } else { 0 };
        (e << 7) | ((period & 0x7) << 4) | (n << 3) | (shift & 0x7)
    }

    #[test]
    fn reload_does_not_apply_sweep_on_same_tick() {
        let mut s = Sweep::new(PulseType::Pulse1);

        // enabled, period=2, negate, shift=1
        s.set(sweep_cfg(true, 2, true, 1));

        let mut timer: u16 = 0x040; // 64

        // first half-frame after write: reload happens, no sweep change
        s.clock(&mut timer);
        assert_eq!(timer, 0x040); // 64
        assert_eq!(s.divider(), 2);

        // next two half-frames just count down: 2->1->0, still no sweep
        s.clock(&mut timer);
        assert_eq!(timer, 0x040); // 64
        assert_eq!(s.divider(), 1);

        s.clock(&mut timer);
        assert_eq!(timer, 0x040); // 64
        assert_eq!(s.divider(), 0);

        // next tick: divider hits 0, sweep applies, divider reloads to period
        s.clock(&mut timer);
        let expected = 0x040 - (0x040 >> 1) - 1; // pulse1 negate quirk
        assert_eq!(timer, expected);
        assert_eq!(s.divider(), 2);
    }

    #[test]
    fn pulse1_vs_pulse2_negate_difference() {
        // period doesn't matter for forced clocking here
        let cfg = sweep_cfg(true, 0, true, 1);

        // Pulse1: timer - (timer>>shift) - 1
        let mut s1 = Sweep::new(PulseType::Pulse1);
        s1.set(cfg);
        let mut t1: u16 = 1000;
        s1.clock(&mut t1); // reload
        // force divider to 0 by clocking period+1 times with period=0 => next tick sweeps immediately
        s1.clock(&mut t1); // divider==0 => sweep
        assert_eq!(t1, 1000 - (1000 >> 1) - 1);

        // Pulse2: timer - (timer>>shift)
        let mut s2 = Sweep::new(PulseType::Pulse2);
        s2.set(cfg);
        let mut t2: u16 = 1000;
        s2.clock(&mut t2); // reload
        s2.clock(&mut t2); // divider==0 => sweep
        assert_eq!(t2, 1000 - (1000 >> 1));
    }

    #[test]
    fn sweep_does_not_update_timer_if_target_invalid() {
        // Target < 8 should not update
        let mut s = Sweep::new(PulseType::Pulse1);
        s.set(sweep_cfg(true, 0, true, 1)); // enabled negate shift1

        let mut timer: u16 = 9; // change=4 => target = 9-4-1 = 4 (invalid)
        s.clock(&mut timer); // reload
        s.clock(&mut timer); // sweep attempt
        assert_eq!(timer, 9); // no timer change

        // Target > 0x7FF should not update
        let mut s = Sweep::new(PulseType::Pulse1);
        s.set(sweep_cfg(true, 0, false, 1)); // enabled add shift1

        // timer = 0x700 = 0b111_0000_0000
        // change = 0x700 >> 1 = 0b11_1000_0000 => 0x380
        // target = 0x700 + 0x380 = 0xA80 (>0x7FF means no sweep)
        let mut timer: u16 = 0x700;
        s.clock(&mut timer); // reload
        s.clock(&mut timer); // sweep attempt
        assert_eq!(timer, 0x700); // no timer change
    }

    #[test]
    fn muting_rules_match_expectations() {
        let mut s = Sweep::new(PulseType::Pulse1);

        // mute for timer < 8
        s.set(sweep_cfg(false, 0, false, 0));
        assert!(s.is_muting(0));
        assert!(s.is_muting(7));
        assert!(!s.is_muting(8));

        // Overflow muting only when enabled and shift>0
        s.set(sweep_cfg(false, 0, false, 1)); // disabled
        assert!(!s.is_muting(0x700)); // would overflow if enabled, but disabled => not muting

        s.set(sweep_cfg(true, 0, false, 0)); // shift=0 => no sweep => not muting
        assert!(!s.is_muting(0x700));

        s.set(sweep_cfg(true, 0, false, 1)); // enabled, shift=1 => overflow
        assert!(s.is_muting(0x700));
    }
}
