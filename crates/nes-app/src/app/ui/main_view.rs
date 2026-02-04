use crate::shared::frame_buffer::SharedFrame;
use eframe::epaint::textures::TextureOptions;
use eframe::epaint::{ColorImage, TextureHandle};
use egui::Context;
use nes_core::prelude::NES_SYSTEM_PALETTE;

pub fn render(
    ctx: &Context,
    texture: &mut Option<TextureHandle>,
    frame: &SharedFrame,
    paused: bool,
) {
    // egui::CentralPanel::default().show(ctx, |ui| {
    //     let pixels = frame.read();
    //     let mut rgba = Vec::with_capacity(256 * 240 * 4);
    //     for &palette_idx in pixels.iter() {
    //         let color = NES_SYSTEM_PALETTE[palette_idx as usize];
    //         rgba.extend_from_slice(&[color.0, color.1, color.2, 255]);
    //     }
    //
    //     let color_image = ColorImage::from_rgba_unmultiplied([256, 240], &rgba);
    //     let tex = texture.get_or_insert_with(|| {
    //         ui.ctx()
    //             .load_texture("nes_frame", color_image.clone(), TextureOptions::LINEAR)
    //     });
    //     tex.set(color_image, TextureOptions::LINEAR);
    //
    //     // Scale while maintaining aspect
    //     let avail = ui.available_size();
    //     let aspect = 256.0 / 240.0;
    //     let (w, h) = if avail.x / avail.y > aspect {
    //         (avail.y * aspect, avail.y)
    //     } else {
    //         (avail.x, avail.x / aspect)
    //     };
    //
    //     ui.image((tex.id(), egui::vec2(w, h)));
    // });
    //
    // if paused {
    //     egui::Window::new("Paused")
    //         .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
    //         .collapsible(false)
    //         .resizable(false)
    //         .show(ctx, |ui| {
    //             ui.label("Press P to unpause");
    //         });
    // }
}
