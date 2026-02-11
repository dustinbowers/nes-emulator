use crate::app::action::Action;
use crate::app::app::UiCtx;
use crate::app::ui::views::UiView;
use crate::emu::commands::AudioChannel;
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

pub fn handle_hotkeys(egui_ctx: &egui::Context, ui_ctx: &mut UiCtx, view: &UiView) {
    if egui_ctx.wants_keyboard_input() {
        return;
    }

    if !ui_ctx.started {
        return;
    }

    let input = egui_ctx.input(|i| i.clone());

    // Play screen hotkeys
    if matches!(view, UiView::Playing(..)) {
        if input.key_pressed(egui::Key::P) {
            ui_ctx.actions.push(Action::TogglePause);
        }

        if input.key_pressed(egui::Key::Num1) {
            ui_ctx
                .actions
                .push(Action::ToggleAudioChannel(AudioChannel::Pulse1));
        }
        if input.key_pressed(egui::Key::Num2) {
            ui_ctx
                .actions
                .push(Action::ToggleAudioChannel(AudioChannel::Pulse2));
        }
        if input.key_pressed(egui::Key::Num3) {
            ui_ctx
                .actions
                .push(Action::ToggleAudioChannel(AudioChannel::Triangle));
        }
        if input.key_pressed(egui::Key::Num4) {
            ui_ctx
                .actions
                .push(Action::ToggleAudioChannel(AudioChannel::Noise));
        }
        if input.key_pressed(egui::Key::Num5) {
            ui_ctx
                .actions
                .push(Action::ToggleAudioChannel(AudioChannel::DMC));
        }
    }
}
