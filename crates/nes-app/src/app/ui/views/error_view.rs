use crate::app::action::Action;
use crate::app::app::UiCtx;
use crate::app::ui::error::ErrorInfo;

pub struct ErrorView {
    info: ErrorInfo,
    show_details: bool,
    details_cache: String,
}

impl ErrorView {
    pub fn new(info: ErrorInfo) -> Self {
        let details_cache = info.details.clone();
        Self {
            info,
            show_details: true,
            details_cache,
        }
    }

    pub fn ui(&mut self, egui_ctx: &egui::Context, ui_ctx: &mut UiCtx) {
        egui::CentralPanel::default().show(egui_ctx, |_ui| {
            egui::Area::new("error_panel".into())
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(egui_ctx, |ui| {
                    ui.set_min_size(egui::vec2(420.0, 360.0));

                    egui::Frame::group(ui.style())
                        .inner_margin(egui::Margin::symmetric(28, 28))
                        .corner_radius(egui::CornerRadius::same(12))
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.heading("Error");
                                ui.add_space(10.0);

                                // Big “at a glance” context
                                ui.label(
                                    egui::RichText::new(&self.info.context).size(18.0).strong(),
                                );

                                ui.add_space(8.0);
                                ui.label(
                                    egui::RichText::new(
                                        "You can copy details to share in a bug report.",
                                    )
                                    .color(ui.visuals().weak_text_color()),
                                );

                                ui.add_space(14.0);
                                ui.separator();
                                ui.add_space(10.0);

                                // Details toggle + actions
                                ui.horizontal(|ui| {
                                    let label = if self.show_details {
                                        "Hide details"
                                    } else {
                                        "Show details"
                                    };
                                    if ui.button(label).clicked() {
                                        self.show_details = !self.show_details;
                                    }

                                    ui.add_space(8.0);

                                    if ui.button("Copy").clicked() {
                                        let text = format!(
                                            "{}\n\n{}",
                                            self.info.context, self.info.details
                                        );
                                        // TODO: Ensure support for wasm...
                                        egui_ctx.copy_text(text);
                                    }
                                });

                                if self.show_details {
                                    ui.add_space(10.0);

                                    ui.add(
                                        egui::TextEdit::multiline(&mut self.details_cache)
                                            .font(egui::TextStyle::Monospace)
                                            .desired_width(f32::INFINITY)
                                            .desired_rows(8)
                                            .interactive(false),
                                    );
                                }

                                ui.add_space(14.0);
                                ui.separator();
                                ui.add_space(12.0);

                                // Acknowledge row
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .add_sized([120.0, 34.0], egui::Button::new("OK"))
                                            .clicked()
                                        {
                                            ui_ctx.actions.push(Action::AcknowledgeError);
                                        }
                                    },
                                );
                            });
                        });
                });
        });
    }
}
