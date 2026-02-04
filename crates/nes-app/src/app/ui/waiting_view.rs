use crate::app::app::App;
use crate::emu::host::EmuHost;

pub fn render(ctx: &egui::Context) -> Option<Vec<u8>>{
    let mut rom_data: Option<Vec<u8>> = None;
    egui::CentralPanel::default().show(ctx, |ui| {
        let available = ui.available_size();
        let panel_width = 420.0;
        let panel_height = 240.0; // approximate height frame

        // Compute the top-left of the centered rect
        let top_left = egui::pos2(
            (available.x - panel_width) / 2.0,
            (available.y - panel_height) / 2.0,
        );

        let rect =
            egui::Rect::from_min_size(top_left, egui::vec2(panel_width, panel_height));

        ui.allocate_ui_at_rect(rect, |ui| {
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::symmetric(32, 32))
                .rounding(egui::Rounding::same(12))
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
                                rom_data = Some(rom_bytes);
                            }

                            ui.add_space(16.0);
                        }

                        ui.label(
                            egui::RichText::new("Or drag & drop a .nes file")
                                .size(16.0),
                        );
                    });
                });
        });
    });
    rom_data
}