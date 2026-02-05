use crate::trace_ppu_event;

pub enum NmiEvent {
    /// PPU enters vblank
    VBlankSet,
    /// PPU exits vblank
    VBlankCleared,
    /// $2000 bit 7 went 0->1
    NmiEnableSet,
    /// $2000 bit 7 went 1->0
    NmiEnableCleared,
    /// $2002 read at the vblank-set edge window
    StatusReadDuringVBlankSet,
    /// $2002 read cleared the vblank flag (PPUSTATUS bit 7)
    StatusReadClearsVBlank,
}

/// PPU-side NMI line model
#[derive(Debug, Default)]
pub struct Nmi {
    enabled: bool,
    vblank: bool,

    /// Set by a $2002 read in the vblank-set window (consumed on VBlankSet)
    suppress_next_vblank_edge: bool,

    /// NMI output line level driven by the PPU
    line: bool,
}

impl Nmi {
    // pub fn reset(&mut self) {
    //     *self = Self::default();
    // }

    /// Current NMI output line level
    #[inline]
    pub fn line(&self) -> bool {
        self.line
    }

    #[inline]
    fn set_line(&mut self, high: bool, why: &'static str) {
        if self.line != high {
            self.line = high;
            trace_ppu_event!(
                "[NMI LINE {}] why={} enabled={} vblank={} suppress_next_edge={}",
                if high { "HIGH" } else { "LOW" },
                why,
                self.enabled,
                self.vblank,
                self.suppress_next_vblank_edge
            );
        }
    }

    pub fn on_event(&mut self, event: NmiEvent) {
        match event {
            NmiEvent::VBlankSet => {
                self.vblank = true;

                if self.enabled && !self.suppress_next_vblank_edge {
                    self.set_line(true, "vblank_entry");
                } else {
                    self.set_line(false, "vblank_entry_suppressed_or_disabled");
                }

                self.suppress_next_vblank_edge = false;
            }
            NmiEvent::VBlankCleared => {
                self.vblank = false;
                self.set_line(false, "vblank_exit");
                self.suppress_next_vblank_edge = false;
            }
            NmiEvent::NmiEnableSet => {
                self.enabled = true;

                if self.vblank {
                    self.set_line(true, "enable_during_vblank");
                } else {
                    self.set_line(false, "enable_outside_vblank");
                }
            }
            NmiEvent::NmiEnableCleared => {
                self.enabled = false;
                self.set_line(false, "disable");
            }
            NmiEvent::StatusReadDuringVBlankSet => {
                self.suppress_next_vblank_edge = true;
            }
            NmiEvent::StatusReadClearsVBlank => {
                // Reading $2002 clears the vblank flag; dropping vblank should drop the line
                self.vblank = false;
                self.set_line(false, "status_read_clears_vblank");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_line(nmi: &Nmi, expected: bool, msg: &str) {
        assert_eq!(nmi.line(), expected, "{}", msg);
    }

    #[test]
    fn vblank_entry_asserts_line_when_enabled() {
        let mut nmi = Nmi::default();

        nmi.on_event(NmiEvent::NmiEnableSet);
        assert_line(
            &nmi,
            false,
            "enabling outside vblank should not assert NMI line",
        );

        nmi.on_event(NmiEvent::VBlankSet);
        assert_line(
            &nmi,
            true,
            "vblank entry with NMI enabled should assert line",
        );
    }

    #[test]
    fn vblank_entry_does_not_assert_line_when_disabled() {
        let mut nmi = Nmi::default();

        nmi.on_event(NmiEvent::VBlankSet);
        assert_line(
            &nmi,
            false,
            "vblank entry with NMI disabled should keep line low",
        );
    }

    #[test]
    fn status_read_during_vblank_set_suppresses_next_vblank_edge_only() {
        let mut nmi = Nmi::default();

        nmi.on_event(NmiEvent::StatusReadDuringVBlankSet);

        nmi.on_event(NmiEvent::NmiEnableSet);
        nmi.on_event(NmiEvent::VBlankSet);
        assert_line(
            &nmi,
            false,
            "suppression should prevent line asserting on this vblank entry",
        );

        nmi.on_event(NmiEvent::VBlankCleared);
        assert_line(&nmi, false, "line remains low after vblank exit");

        nmi.on_event(NmiEvent::VBlankSet);
        assert_line(
            &nmi,
            true,
            "suppression is one-shot; next vblank entry should assert line",
        );
    }

    #[test]
    fn enabling_during_vblank_asserts_line_immediately() {
        let mut nmi = Nmi::default();

        nmi.on_event(NmiEvent::VBlankSet);
        assert_line(&nmi, false, "entering vblank while disabled keeps line low");

        nmi.on_event(NmiEvent::NmiEnableSet);
        assert_line(
            &nmi,
            true,
            "enabling during vblank should assert line immediately",
        );
    }

    #[test]
    fn disabling_during_vblank_deasserts_line() {
        let mut nmi = Nmi::default();

        nmi.on_event(NmiEvent::NmiEnableSet);
        nmi.on_event(NmiEvent::VBlankSet);
        assert_line(&nmi, true, "line high in vblank when enabled");

        nmi.on_event(NmiEvent::NmiEnableCleared);
        assert_line(&nmi, false, "disabling should drop NMI line immediately");

        nmi.on_event(NmiEvent::NmiEnableSet);
        assert_line(
            &nmi,
            true,
            "re-enabling during vblank should re-assert line",
        );
    }

    #[test]
    fn vblank_exit_deasserts_line() {
        let mut nmi = Nmi::default();

        nmi.on_event(NmiEvent::NmiEnableSet);
        nmi.on_event(NmiEvent::VBlankSet);
        assert_line(&nmi, true, "line high during vblank when enabled");

        nmi.on_event(NmiEvent::VBlankCleared);
        assert_line(&nmi, false, "vblank exit should drop NMI line");
    }

    #[test]
    fn status_read_clears_vblank_drops_line() {
        let mut nmi = Nmi::default();

        nmi.on_event(NmiEvent::NmiEnableSet);
        nmi.on_event(NmiEvent::VBlankSet);
        assert_line(&nmi, true, "line high during vblank when enabled");

        nmi.on_event(NmiEvent::StatusReadClearsVBlank);
        assert_line(
            &nmi,
            false,
            "clearing vblank via status read should drop line",
        );
    }

    #[test]
    fn vblank_cleared_resets_suppression() {
        let mut nmi = Nmi::default();

        nmi.on_event(NmiEvent::StatusReadDuringVBlankSet);
        nmi.on_event(NmiEvent::VBlankCleared);

        nmi.on_event(NmiEvent::NmiEnableSet);
        nmi.on_event(NmiEvent::VBlankSet);
        assert_line(
            &nmi,
            true,
            "suppression should be cleared by VBlankCleared; vblank entry should assert",
        );
    }

    #[test]
    fn status_read_during_vblank_set_does_not_drop_current_line() {
        let mut nmi = Nmi::default();

        nmi.on_event(NmiEvent::NmiEnableSet);
        nmi.on_event(NmiEvent::VBlankSet);
        assert_line(&nmi, true, "line asserted on vblank entry");

        nmi.on_event(NmiEvent::StatusReadDuringVBlankSet);
        assert_line(
            &nmi,
            true,
            "arming suppression during vblank should not drop an already-high line",
        );

        nmi.on_event(NmiEvent::VBlankCleared);
        assert_line(&nmi, false, "line drops on vblank exit");

        nmi.on_event(NmiEvent::VBlankSet);
        assert_line(
            &nmi,
            true,
            "suppression does not persist across vblank exit; next vblank entry asserts",
        );

        nmi.on_event(NmiEvent::VBlankCleared);
        nmi.on_event(NmiEvent::VBlankSet);
        assert_line(
            &nmi,
            true,
            "suppression is one-shot; subsequent vblank asserts",
        );
    }
}
