#[cfg(not(target_arch = "wasm32"))]
use crate::app::action::Action;
use crate::app::app::UiCtx;

pub struct RomSelectView {
    pub rom_bytes: Option<Vec<u8>>,
}

impl Default for RomSelectView {
    fn default() -> Self {
        Self::new()
    }
}

impl RomSelectView {
    pub fn new() -> Self {
        RomSelectView { rom_bytes: None }
    }

    pub fn ui(&mut self, egui_ctx: &egui::Context, ui_ctx: &mut UiCtx) {
        egui::CentralPanel::default().show(egui_ctx, |_ui| {
            egui::Area::new("rom_select_panel".into())
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(egui_ctx, |ui| {
                    // Fixed dialog size
                    ui.set_min_size(egui::vec2(420.0, 240.0));

                    egui::Frame::group(ui.style())
                        .inner_margin(egui::Margin::symmetric(32, 32))
                        .corner_radius(egui::CornerRadius::same(12))
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.heading("Insert a Cartridge");
                                ui.add_space(12.0);

                                ui.label(
                                    egui::RichText::new("Load a NES ROM to begin playing")
                                        .color(ui.visuals().weak_text_color()),
                                );

                                ui.add_space(24.0);

                                #[cfg(not(target_arch = "wasm32"))]
                                {
                                    if ui
                                        .add_sized(
                                            [200.0, 36.0],
                                            egui::Button::new("Browse for ROM…"),
                                        )
                                        .clicked()
                                        && let Some(path) = rfd::FileDialog::new()
                                            .add_filter("NES ROM", &["nes"])
                                            .pick_file()
                                        && let Ok(rom_bytes) = std::fs::read(path)
                                    {
                                        ui_ctx.actions.push(Action::PlayRom(rom_bytes));
                                    }

                                    ui.add_space(16.0);
                                }

                                ui.label(
                                    egui::RichText::new("Or drag & drop a .nes file").size(16.0),
                                );
                            });
                        });
                });
        });
    }
}
