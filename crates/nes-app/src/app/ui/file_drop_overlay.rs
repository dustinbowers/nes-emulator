use crate::app::action::Action;
use crate::app::app::UiCtx;

pub fn handle_file_drop(ctx: &egui::Context, ui_ctx: &mut UiCtx) {
    // Preview hovering files
    if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
        use egui::*;

        let painter =
            ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

        let content_rect = ctx.content_rect();
        painter.rect_filled(content_rect, 0.0, Color32::from_black_alpha(192));
        painter.text(
            content_rect.center(),
            Align2::CENTER_CENTER,
            "Drop ROM file here",
            FontId::proportional(40.0),
            Color32::WHITE,
        );
    }

    // Handle dropped files
    ctx.input(|i| {
        if !i.raw.dropped_files.is_empty()
            && let Some(file) = i.raw.dropped_files.first()
        {
            #[cfg(not(target_arch = "wasm32"))]
            {
                if let Some(path) = &file.path
                    && let Ok(rom_data) = std::fs::read(path)
                {
                    ui_ctx.actions.push(Action::PlayRom(rom_data));
                }
            }

            #[cfg(target_arch = "wasm32")]
            {
                if let Some(bytes) = &file.bytes {
                    let rom_data = bytes.to_vec();
                    ui_ctx.actions.push(Action::PlayRom(rom_data));
                }
            }
        }
    });
}
