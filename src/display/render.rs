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
        let palette = bg_palette(ppu, tile_column, tile_row);

        let tile_column = i % 32;
        let tile_row = i / 32;

        for y in 0..8 {
            let upper = tile[y];
            let lower = tile[y + 8];

            for x in (0..8).rev() {
                let value = ((lower >> x) & 1) << 1 | ((upper >> x) & 1); // Build pixel value
                let color = match value {
                    0 => COLOR_MAP.get_color(palette[0] as usize),
                    1 => COLOR_MAP.get_color(palette[1] as usize),
                    2 => COLOR_MAP.get_color(palette[1] as usize),
                    3 => COLOR_MAP.get_color(palette[1] as usize),
                    _ => panic!("Impossible color index"),
                };
                frame.set_pixel(tile_column * 8 + x, tile_row * 8 + y, *color)
            }
        }
    }
}

fn render_name_table(ppu: &PPU, frame: &mut Frame, name_table: &[u8]) {
    let bank = ppu.ctrl_register.background_pattern_addr();


}

fn bg_palette(ppu: &PPU, tile_column: usize, tile_row: usize) -> [u8; 4] {
    let attr_table_idx = tile_row / 4 * 8 +  tile_column / 4;
    let attr_byte = ppu.ram[0x3c0 + attr_table_idx];  // note: still using hardcoded first nametable

    let pallet_idx = match (tile_column % 4 / 2, tile_row % 4 / 2) {
        (0, 0) => (attr_byte >> 0) & 0b11,
        (1, 0) => (attr_byte >> 2) & 0b11,
        (0, 1) => (attr_byte >> 4) & 0b11,
        (1, 1) => (attr_byte >> 6) & 0b11,
        (_, _) => panic!("invalid pallet_idx"),
    };

    let palette_start: usize = 1 + (pallet_idx as usize) * 4;

    [
        ppu.palette_table[0],
        ppu.palette_table[palette_start],
        ppu.palette_table[palette_start + 1],
        ppu.palette_table[palette_start + 2],
    ]
}
