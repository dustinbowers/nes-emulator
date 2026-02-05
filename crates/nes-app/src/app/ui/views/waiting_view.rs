use crate::app::action::Action;
use crate::app::app::UiCtx;
use egui::Ui;

pub struct WaitingView;

impl Default for WaitingView {
    fn default() -> Self {
        Self::new()
    }
}

impl WaitingView {
    pub fn new() -> Self {
        WaitingView
    }

    pub fn ui(&mut self, egui_ctx: &egui::Context, ui_ctx: &mut UiCtx) {
        egui::CentralPanel::default().show(egui_ctx, |_| {
            egui::Area::new("waiting_panel".into())
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(egui_ctx, |ui: &mut Ui| {
                    egui::Frame::group(ui.style())
                        .inner_margin(egui::Margin::symmetric(32, 32))
                        .corner_radius(egui::CornerRadius::same(12))
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.heading("Start Emulator");
                                ui.add_space(12.0);

                                ui.label(
                                    egui::RichText::new(
                                        "Audio requires a user interaction in the browser.\nClick to start the emulator.",
                                    )
                                        .color(ui.visuals().weak_text_color()),
                                );

                                ui.add_space(24.0);

                                let button = egui::Button::new(
                                    egui::RichText::new("Click to Start")
                                        .size(18.0)
                                        .strong(),
                                )
                                    .min_size(egui::vec2(220.0, 44.0));

                                if ui.add(button).clicked() {
                                    ui_ctx.actions.push(Action::Start);
                                }

                                ui.add_space(16.0);

                                ui.label(
                                    egui::RichText::new(
                                        "After starting, you can drag & drop a .nes file.",
                                    )
                                        .color(ui.visuals().weak_text_color()),
                                );
                            });
                        });
                });
        });
    }
}
