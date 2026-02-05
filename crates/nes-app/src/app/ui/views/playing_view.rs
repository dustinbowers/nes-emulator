use crate::app::app::UiCtx;
use eframe::epaint::ColorImage;
use eframe::epaint::textures::TextureOptions;
use nes_core::prelude::NES_SYSTEM_PALETTE;

pub struct PlayingView {}

impl Default for PlayingView {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayingView {
    pub fn new() -> Self {
        PlayingView {}
    }

    pub fn ui(&mut self, egui_ctx: &egui::Context, ui_ctx: &mut UiCtx) {
        egui::CentralPanel::default().show(egui_ctx, |ui| {
            let pixels = ui_ctx.frame.read();
            let mut rgba = Vec::with_capacity(256 * 240 * 4);
            for &palette_idx in pixels.iter() {
                let color = NES_SYSTEM_PALETTE[palette_idx as usize];
                rgba.extend_from_slice(&[color.0, color.1, color.2, 255]);
            }

            let color_image = ColorImage::from_rgba_unmultiplied([256, 240], &rgba);
            let tex = ui_ctx.texture.get_or_insert_with(|| {
                ui.ctx()
                    .load_texture("nes_frame", color_image.clone(), TextureOptions::LINEAR)
            });
            tex.set(color_image, TextureOptions::LINEAR);

            // Scale while maintaining aspect
            let avail = ui.available_size();
            let aspect = 256.0 / 240.0;
            let (w, h) = if avail.x / avail.y > aspect {
                (avail.y * aspect, avail.y)
            } else {
                (avail.x, avail.x / aspect)
            };

            ui.image((tex.id(), egui::vec2(w, h)));
        });

        if ui_ctx.paused {
            // Dim the background
            let screen_rect = egui_ctx.content_rect();
            egui::Area::new("paused_dim".into())
                .fixed_pos(screen_rect.min)
                .show(egui_ctx, |ui| {
                    ui.painter().rect_filled(
                        screen_rect,
                        0.0,
                        egui::Color32::from_black_alpha(120),
                    );
                });

            // [Pause] popup
            egui::Area::new("paused_badge".into())
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(egui_ctx, |ui| {
                    egui::Frame::popup(ui.style())
                        .corner_radius(egui::CornerRadius::same(12))
                        .inner_margin(egui::Margin::symmetric(18, 14))
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.label(egui::RichText::new("Paused").strong().size(22.0));
                                ui.add_space(6.0);
                                ui.label(egui::RichText::new("Press P to resume"));
                            });
                        });
                });
        }
    }
}
