use std::sync::Arc;
use eframe::egui_glow;
use crate::app::app::UiCtx;
use nes_core::prelude::NES_SYSTEM_PALETTE;

pub struct PlayingView {
    rgba: Vec<u8>,
}

impl Default for PlayingView {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayingView {
    pub fn new() -> Self {
        PlayingView {
            rgba: vec![0; 256 * 240 * 4],
        }
    }

        pub fn ui(&mut self, egui_ctx: &egui::Context, ui_ctx: &mut UiCtx) {
            egui::CentralPanel::default().show(egui_ctx, |ui| {
                let pixels = ui_ctx.frame.read();
                for (i, &palette_idx) in pixels.iter().enumerate() {
                    let (r, g, b) = NES_SYSTEM_PALETTE[palette_idx as usize];
                    let o = i * 4; //offset
                    self.rgba[o + 0] = r;
                    self.rgba[o + 1] = g;
                    self.rgba[o + 2] = b;
                    self.rgba[o + 3] = 255;
                }

                // scale while maintaining aspect
                let avail = ui.available_size();
                let aspect = 256.0 / 240.0;
                let (w, h) = if avail.x / avail.y > aspect {
                    (avail.y * aspect, avail.y)
                } else {
                    (avail.x, avail.x / aspect)
                };

                let desired = egui::vec2(w, h);
                let (rect, _resp) = ui.allocate_exact_size(desired, egui::Sense::hover());

                unsafe {
                    ui_ctx.post_fx.lock().unwrap().upload_frame(&self.rgba);
                }

                // draw with shader
                let post_fx = ui_ctx.post_fx.clone();
                let time = ui_ctx.time_s;

                ui.painter().add(egui::PaintCallback {
                    rect,
                    callback: Arc::new(egui_glow::CallbackFn::new(move |info, painter| {
                        let gl = painter.gl();
                        let fx = post_fx.lock().unwrap();
                        unsafe {
                            fx.paint(gl, info, time);
                        }
                    })),
                });
            });

        // PAUSED state
        if ui_ctx.paused {
           self.draw_paused(egui_ctx);
        }
    }

    fn draw_paused(&mut self, egui_ctx: &egui::Context) {
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
