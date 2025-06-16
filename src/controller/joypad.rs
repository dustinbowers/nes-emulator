// See: https://www.nesdev.org/wiki/Controller_reading

use super::NesController;
use bitflags::bitflags;

bitflags! {
       #[derive(Copy, Clone, Debug)]
       pub struct JoypadButtons: u8 {
           const BUTTON_A          = 0b0000_0001;
           const BUTTON_B          = 0b0000_0010;
           const SELECT            = 0b0000_0100;
           const START             = 0b0000_1000;
           const UP                = 0b0001_0000;
           const DOWN              = 0b0010_0000;
           const LEFT              = 0b0100_0000;
           const RIGHT             = 0b1000_0000;
       }
}

pub struct Joypad {
    buttons: JoypadButtons,
    button_index: u8,
    strobe: bool,
}

impl NesController for Joypad {
    fn read(&mut self) -> u8 {
        if self.button_index > 7 {
            return 1;
        }
        let status = (self.buttons.bits() >> self.button_index) & 0b1;
        if !self.strobe && self.button_index <= 7 {
            self.button_index += 1;
        }
        status
    }

    fn write(&mut self, data: u8) {
        self.strobe = data & 0b1 == 1;
        match self.strobe {
            true => self.button_index = 0,
            _ => {}
        }
    }
}

impl Joypad {
    pub fn new() -> Self {
        Self {
            buttons: JoypadButtons::from_bits_truncate(0),
            button_index: 0,
            strobe: false,
        }
    }

    pub fn set_button_status(&mut self, button: &JoypadButtons, state: bool) {
        self.buttons.set(button.clone(), state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_joypad_default_state() {
        let joypad = Joypad::new();
        assert_eq!(joypad.buttons.bits(), 0);
        assert_eq!(joypad.strobe, false);
        assert_eq!(joypad.button_index, 0);
    }

    #[test]
    fn test_button_press_and_release() {
        let mut joypad = Joypad::new();

        // Press A and Start
        joypad.set_button_status(&JoypadButtons::BUTTON_A, true);
        joypad.set_button_status(&JoypadButtons::START, true);

        assert!(joypad.buttons.contains(JoypadButtons::BUTTON_A));
        assert!(joypad.buttons.contains(JoypadButtons::START));
        assert!(!joypad.buttons.contains(JoypadButtons::BUTTON_B));

        // Release A
        joypad.set_button_status(&JoypadButtons::BUTTON_A, false);
        assert!(!joypad.buttons.contains(JoypadButtons::BUTTON_A));
    }

    #[test]
    fn test_strobe_behavior() {
        let mut joypad = Joypad::new();

        joypad.set_button_status(&JoypadButtons::BUTTON_A, true);
        joypad.set_button_status(&JoypadButtons::RIGHT, true);

        // Enable strobe
        joypad.write(1);
        assert!(joypad.strobe);
        assert_eq!(joypad.button_index, 0);

        // Reading with strobe = 1 should always return BUTTON_A
        for _ in 0..10 {
            assert_eq!(joypad.read(), 1);
        }

        // Disable strobe
        joypad.write(0);
        assert!(!joypad.strobe);
        assert_eq!(joypad.button_index, 0);

        // Sequential read for each button
        let expected = [
            JoypadButtons::BUTTON_A,
            JoypadButtons::BUTTON_B,
            JoypadButtons::SELECT,
            JoypadButtons::START,
            JoypadButtons::UP,
            JoypadButtons::DOWN,
            JoypadButtons::LEFT,
            JoypadButtons::RIGHT,
        ];

        for i in 0..8 {
            let bit = joypad.read();
            let expected_bit = if joypad.buttons.contains(expected[i].clone()) {
                1
            } else {
                0
            };
            assert_eq!(bit, expected_bit, "Button {:?} read incorrect", expected[i]);
        }

        // Button index beyond 7 should always return 1
        assert_eq!(joypad.read(), 1);
        assert_eq!(joypad.read(), 1);
    }
}
