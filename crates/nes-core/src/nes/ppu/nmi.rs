

pub enum NmiEvent {
    VBlankSet,                  // SL 241, dot 1 (or 0?)
    VBlankCleared,              // SL 261, dot 2 (or 0?)
    NmiEnableSet,               // $2000 write sets bit 7
    NmiEnableCleared,           // $2000 write clears bit 7
    StatusReadDuringVBlankSet   // $2002 read on set edge
}

#[derive(Debug, Default)]
pub struct Nmi {
    enabled: bool,
    vblank: bool,
    fired_this_vblank: bool,
    suppress_next: bool,
    pending: bool,
}

// impl Default for Nmi {
//     fn default() -> Self {
//         Self {
//             enabled: false,
//             vblank: false,
//             fired_this_vblank: false,
//             suppress_next: false,
//             pending: false
//         }
//     }
// }

impl Nmi {
    pub fn on_event(&mut self, ev: NmiEvent) {
        match ev {
            NmiEvent::VBlankSet => {
                self.vblank = true;

                if self.enabled && !self.suppress_next && !self.fired_this_vblank {
                    self.pending = true;
                    self.fired_this_vblank = true;
                }

                self.suppress_next = false;
            }
            NmiEvent::VBlankCleared => {
                self.vblank = false;
                self.fired_this_vblank = false;
                self.pending = false;
            }
            NmiEvent::NmiEnableSet => {
                self.enabled = true;

                // Edge-trigger: enable during vblank causes immediate NMI
                if self.vblank && !self.fired_this_vblank {
                    self.pending = true;
                    self.fired_this_vblank = true;
                }
            }
            NmiEvent::NmiEnableCleared => {
                self.enabled = false;
            }
            NmiEvent::StatusReadDuringVBlankSet => {
                self.suppress_next = true;
            }
        }
    }

    pub fn poll(&mut self) -> bool {
        if self.pending {
            self.pending = false;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_vblank_nmi_fires() {
        let mut nmi = Nmi::default();

        // Enable nmi before vblank
        nmi.on_event(NmiEvent::NmiEnableSet);
        assert!(!nmi.poll());

        // VBlank starts
        nmi.on_event(NmiEvent::VBlankSet);
        assert!(nmi.poll(), "NMI should fire at VBlank start");
        assert!(!nmi.poll(), "NMI should not fire twice");
    }

    #[test]
    fn test_vblank_clear_resets_state() {
        let mut nmi = Nmi::default();

        nmi.on_event(NmiEvent::NmiEnableSet);
        nmi.on_event(NmiEvent::VBlankSet);
        assert!(nmi.poll());

        // Clear VBlank
        nmi.on_event(NmiEvent::VBlankCleared);
        assert!(!nmi.poll(), "No NMI should be pending after clear");

        // Next vblank fires again
        nmi.on_event(NmiEvent::VBlankSet);
        assert!(nmi.poll());
    }

    #[test]
    fn test_enable_during_vblank_fires_immediately() {
        let mut nmi = Nmi::default();

        // VBlank set first, NMI disabled
        nmi.on_event(NmiEvent::VBlankSet);
        assert!(!nmi.poll());

        // Now enable NMI
        nmi.on_event(NmiEvent::NmiEnableSet);
        assert!(nmi.poll(), "Enabling NMI during VBlank should fire immediately");

        // Should not fire twice
        assert!(!nmi.poll());
    }

    #[test]
    fn test_status_read_suppresses_next_nmi() {
        let mut nmi = Nmi::default();

        nmi.on_event(NmiEvent::NmiEnableSet);

        // Read $2002 right before VBlank
        nmi.on_event(NmiEvent::StatusReadDuringVBlankSet);

        // VBlank starts
        nmi.on_event(NmiEvent::VBlankSet);
        assert!(!nmi.poll(), "NMI suppressed due to status read");

        // VBlank clears and next VBlank should fire normally
        nmi.on_event(NmiEvent::VBlankCleared);
        nmi.on_event(NmiEvent::VBlankSet);
        assert!(nmi.poll());
    }

    #[test]
    fn test_disable_nmi_during_vblank_prevents_firing() {
        let mut nmi = Nmi::default();

        nmi.on_event(NmiEvent::NmiEnableSet);

        // Disable NMI before VBlank
        nmi.on_event(NmiEvent::NmiEnableCleared);

        nmi.on_event(NmiEvent::VBlankSet);
        assert!(!nmi.poll(), "NMI should not fire when disabled");

        // Re-enable NMI during VBlank
        nmi.on_event(NmiEvent::NmiEnableSet);
        assert!(nmi.poll(), "Enabling during VBlank triggers NMI");
    }

    #[test]
    fn test_multiple_vblanks_fire_once_each() {
        let mut nmi = Nmi::default();

        nmi.on_event(NmiEvent::NmiEnableSet);

        // First VBlank
        nmi.on_event(NmiEvent::VBlankSet);
        assert!(nmi.poll());
        assert!(!nmi.poll());

        // Still in VBlank, no extra NMI
        nmi.on_event(NmiEvent::VBlankSet);
        assert!(!nmi.poll());

        // Clear VBlank
        nmi.on_event(NmiEvent::VBlankCleared);

        // Second VBlank
        nmi.on_event(NmiEvent::VBlankSet);
        assert!(nmi.poll());
    }
}