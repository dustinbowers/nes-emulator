use crate::display::color_map::COLOR_MAP;
use crate::display::frame::Frame;
use crate::ppu::PPU;

pub fn render(ppu: &PPU, frame: &mut Frame) {
    let bank = ppu.ctrl_register.background_pattern_addr();

    for i in 0..0x3c0 {
        let tile_index = ppu.ram[i] as u16;
        let tile_start = (bank + tile_index * 16) as usize;

        if tile_start + 16 > ppu.chr_rom.len() {
            println!("WARNING: Tile index {} out of bounds!", tile_index);
            continue; // Skip if out of bounds
        }
        let tile = &ppu.chr_rom[tile_start..tile_start + 16];

        let tile_column = i % 32;
        let tile_row = i / 32;

        for y in 0..8 {
            let upper = tile[y];
            let lower = tile[y + 8];

            for x in (0..8).rev() {
                let value = ((lower >> x) & 1) << 1 | ((upper >> x) & 1); // Build pixel value
                let color = match value {
                    0 => COLOR_MAP.get_color(0x01),
                    1 => COLOR_MAP.get_color(0x23),
                    2 => COLOR_MAP.get_color(0x27),
                    3 => COLOR_MAP.get_color(0x30),
                    _ => panic!("Impossible color index"),
                };
                frame.set_pixel(tile_column * 8 + x, tile_row * 8 + y, *color)
            }
        }
    }
}
