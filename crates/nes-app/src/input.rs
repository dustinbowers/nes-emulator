use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Default)]
pub struct GamepadState {
    buttons: AtomicU8,
}

impl GamepadState {
    #[inline]
    pub fn load(&self) -> u8 {
        self.buttons.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set(&self, mask: u8) {
        self.buttons.store(mask, Ordering::Relaxed);
    }
}
