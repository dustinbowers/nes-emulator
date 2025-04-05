use crate::display::color_map::COLOR_MAP;
use crate::display::frame::Frame;
use crate::ppu::PPU;
use crate::rom::Mirroring;

struct ViewPort {
    x1: usize,
    y1: usize,
    x2: usize,
    y2: usize,
}
impl ViewPort {
    pub fn new(x1: usize, y1: usize, x2: usize, y2: usize) -> Self {
        Self { x1, y1, x2, y2 }
    }
}

pub fn render(ppu: &PPU, frame: &mut Frame) {
    let scroll_x = ppu.scroll_register.scroll_x as usize;
    let scroll_y = ppu.scroll_register.scroll_y as usize;

    let (main_nametable, second_nametable) = match ppu.mirroring {
        Mirroring::Vertical => match ppu.ctrl_register.get_nametable_addr() {
            0x2000 | 0x2800 => (&ppu.ram[0..0x400], &ppu.ram[0x400..0x800]),
            0x2400 | 0x2C00 => (&ppu.ram[0x400..0x800], &ppu.ram[0..0x400]),
            _ => panic!(
                "Unexpected nametable address: {:04X}",
                ppu.ctrl_register.get_nametable_addr()
            ),
        },
        Mirroring::Horizontal => match ppu.ctrl_register.get_nametable_addr() {
            0x2000 | 0x2400 => (&ppu.ram[0..0x400], &ppu.ram[0x400..0x800]),
            0x2800 | 0x2C00 => (&ppu.ram[0x400..0x800], &ppu.ram[0..0x400]),
            _ => panic!(
                "Unexpected nametable address: {:04X}",
                ppu.ctrl_register.get_nametable_addr()
            ),
        },
        _ => panic!("Unsupported mirroring type: {:?}", ppu.mirroring),
    };

    // Render background nametable
    render_name_table(
        ppu,
        frame,
        main_nametable,
        ViewPort::new(scroll_x, scroll_y, 256, 240),
        -(scroll_x as isize),
        -(scroll_y as isize),
    );

    if scroll_x > 0 {
        render_name_table(
            ppu,
            frame,
            second_nametable,
            ViewPort::new(0, 0, scroll_x, 240),
            (256 - scroll_x) as isize,
            0,
        );
    } else if scroll_y > 0 {
        render_name_table(
            ppu,
            frame,
            second_nametable,
            ViewPort::new(0, 0, 256, 240),
            0,
            (240 - scroll_y) as isize,
        );
    }

    // Render sprites
    for i in (0..ppu.oam_data.len()).step_by(4).rev() {
        let tile_y = ppu.oam_data[i + 0] as usize;
        let tile_index = ppu.oam_data[i + 1] as u16;
        let tile_attributes = ppu.oam_data[i + 2];
        let tile_x = ppu.oam_data[i + 3] as usize;

        let flip_vertical = tile_attributes >> 7 & 1 == 1;
        let flip_horizontal = tile_attributes >> 6 & 1 == 1;
        let palette_index = tile_attributes & 0b11;
        let sprite_palette = get_sprite_palette(ppu, palette_index);

        let is_8x16 = ppu.ctrl_register.sprite_size() == 16;
        let bank_addr = if is_8x16 {
            // (tile_index & 1) * 0x1000 // Select correct CHR-ROM bank
            unimplemented!("is_8x16 sprite!");
        } else {
            ppu.ctrl_register.sprite_pattern_addr()
        } as usize;
        let tile_chr_index = bank_addr + ((tile_index & 0xFE) as usize * 16);
        let tile = &ppu.chr_rom[tile_chr_index..tile_chr_index + 16];

        for y in 0..8 {
            let upper = tile[y];
            let lower = tile[y + 8];
            for x in 0..8 {
                let pixel_x = if flip_horizontal { 7 - x } else { x };
                let pixel_y = if flip_vertical { 7 - y } else { y };

                let msb = (lower >> (7 - x)) & 1;
                let lsb = (upper >> (7 - x)) & 1;
                let palette_idx = (msb << 1) | lsb;
                if palette_idx == 0 {
                    continue;
                }

                let color = if ppu.mask_register.show_sprites() {
                    let mut color_index = sprite_palette[palette_idx as usize] as usize;
                    if ppu.mask_register.is_grayscale() {
                        color_index &= 0x30
                    }
                    COLOR_MAP.get_color(color_index)
                } else {
                    continue;
                };

                let screen_x = tile_x + pixel_x;
                let screen_y = tile_y + pixel_y;
                frame.set_pixel(screen_x, screen_y, *color);
            }
        }
    }
}

fn render_name_table(
    ppu: &PPU,
    frame: &mut Frame,
    name_table: &[u8],
    view_port: ViewPort,
    offset_x: isize,
    offset_y: isize,
) {
    // Render Background tiles
    let bank = ppu.ctrl_register.background_pattern_addr();
    let attribute_table = &name_table[0x3c0..0x400];

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
        let palette = get_bg_palette(ppu, attribute_table, tile_column, tile_row);

        let tile_column = i % 32;
        let tile_row = i / 32;

        for y in 0..8 {
            let upper = tile[y];
            let lower = tile[y + 8];

            for x in (0..8).rev() {
                let value = ((lower >> (7 - x)) & 1) << 1 | ((upper >> (7 - x)) & 1); // Build pixel value

                if !ppu.mask_register.show_background() {
                    continue;
                }

                let mut color_index = match value {
                    0 => palette[0],
                    1 => palette[1],
                    2 => palette[2],
                    3 => palette[3],
                    _ => panic!("Impossible color index"),
                } as usize;

                if ppu.mask_register.is_grayscale() {
                    color_index &= 0x30;
                }

                let color = COLOR_MAP.get_color(color_index);
                let px = tile_column * 8 + x;
                let py = tile_row * 8 + y;
                // frame.set_pixel(tile_column * 8 + x, tile_row * 8 + y, *color)
                if px >= view_port.x1
                    && px <= view_port.x2
                    && py >= view_port.y1
                    && py <= view_port.y2
                {
                    let px_shifted = (px as isize + offset_x) as usize;
                    let py_shifted = (py as isize + offset_y) as usize;
                    frame.set_pixel(px_shifted, py_shifted, *color);
                }
            }
        }
    }
}

fn get_bg_palette(
    ppu: &PPU,
    attribute_table: &[u8],
    tile_column: usize,
    tile_row: usize,
) -> [u8; 4] {
    let attr_table_idx = (tile_row / 4) * 8 + tile_column / 4;
    let attr_byte = attribute_table[attr_table_idx];

    let pallet_bit_1 = tile_column % 4 / 2;
    let pallet_bit_0 = tile_row % 4 / 2;
    let pallet_idx = match (pallet_bit_1, pallet_bit_0) {
        (0, 0) => (attr_byte >> 0) & 0b11,
        (1, 0) => (attr_byte >> 2) & 0b11,
        (0, 1) => (attr_byte >> 4) & 0b11,
        (1, 1) => (attr_byte >> 6) & 0b11,
        (_, _) => panic!("impossible pallet_idx"),
    };

    let palette_start: usize = 0x01 + (pallet_idx as usize) * 4;

    [
        ppu.palette_table[0],
        ppu.palette_table[palette_start],
        ppu.palette_table[palette_start + 1],
        ppu.palette_table[palette_start + 2],
    ]
}

fn get_sprite_palette(ppu: &PPU, palette_idx: u8) -> [u8; 4] {
    let start = 0x11 + (palette_idx * 4) as usize;
    [
        0,
        ppu.palette_table[start],
        ppu.palette_table[start + 1],
        ppu.palette_table[start + 2],
    ]
}
