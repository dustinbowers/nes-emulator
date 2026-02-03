use crate::emu::host::EmuHost;
use nes_core::prelude::JoypadButton;

pub fn update_controller_state(ctx: &egui::Context, emu: &EmuHost) {
    let p1_key_map: &[(egui::Key, JoypadButton)] = &[
        (egui::Key::K, JoypadButton::BUTTON_A),
        (egui::Key::J, JoypadButton::BUTTON_B),
        (egui::Key::Enter, JoypadButton::START),
        (egui::Key::Space, JoypadButton::SELECT),
        (egui::Key::W, JoypadButton::UP),
        (egui::Key::S, JoypadButton::DOWN),
        (egui::Key::A, JoypadButton::LEFT),
        (egui::Key::D, JoypadButton::RIGHT),
    ];

    let mut p1 = 0u8;
    for (key, button) in p1_key_map.iter() {
        if ctx.input(|i| i.key_down(*key)) {
            p1 |= button.bits();
        }
    }

    let p2 = 0u8;
    // TODO Maybe later: p2 input

    emu.set_input(p1, p2);
}
