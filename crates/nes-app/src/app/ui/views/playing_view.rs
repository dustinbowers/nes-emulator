use crate::app::app::{UiCtx};
use crate::app::event::AppEventSource;
use crate::app::ui::views::UiView;
use eframe::epaint::ColorImage;
use eframe::epaint::textures::TextureOptions;
use nes_core::prelude::NES_SYSTEM_PALETTE;
use crate::app::ui::Transition;

pub struct PlayingView {
    pub(crate) paused: bool,
}

impl PlayingView {
    pub fn new() -> Self {
        PlayingView { paused: false }
    }

    pub fn ui<E: AppEventSource>(
        &mut self,
        egui_ctx: &egui::Context,
        ui_ctx: &mut UiCtx,
    ) -> Transition {
        let mut transition = Transition::None;
        let Some(frame) = ui_ctx.frame.as_ref() else {
            return Transition::to(UiView::error_shared_frame());
        };

        egui::CentralPanel::default().show(egui_ctx, |ui| {
            let pixels = frame.read();
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

        transition
    }
}
