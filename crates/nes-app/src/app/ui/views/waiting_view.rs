use crate::app::action::Action;
use crate::app::app::UiCtx;

pub struct WaitingView;

impl WaitingView {
    pub fn new() -> Self {
        WaitingView
    }

    pub fn ui(&mut self, egui_ctx: &egui::Context, ui_ctx: &mut UiCtx) {
        egui::CentralPanel::default().show(egui_ctx, |ui| {
            let available = ui.available_size();
            let panel_width = 420.0;
            let panel_height = 240.0;

            let top_left = egui::pos2(
                (available.x - panel_width) / 2.0,
                (available.y - panel_height) / 2.0,
            );
            let rect = egui::Rect::from_min_size(top_left, egui::vec2(panel_width, panel_height));

            ui.allocate_ui_at_rect(rect, |ui| {
                egui::Frame::group(ui.style())
                    .inner_margin(egui::Margin::symmetric(32, 32))
                    .rounding(egui::Rounding::same(12))
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
                                egui::RichText::new("After starting, you can drag & drop a .nes file.")
                                    .color(ui.visuals().weak_text_color()),
                            );
                        });
                    });
            });
        });
    }
}
