
pub fn handle_file_drop<F>(ctx: &egui::Context, mut drop_callback: F)
where
    F: FnMut(Vec<u8>)
{
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
                    drop_callback(rom_data);
                }
            }

            #[cfg(target_arch = "wasm32")]
            {
                if let Some(bytes) = &file.bytes {
                    drop_callback(bytes.to_vec());
                }
            }
        }
    });
}