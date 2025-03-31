use crate::display::color_map::COLOR_MAP;
use crate::display::frame::Frame;
use crate::ppu::PPU;

// pub fn render(ppu: &PPU, frame: &mut Frame) {
//     let bank = ppu.ctrl_register.background_pattern_addr();
//
//     for i in 0..0x03c0 { // use the first nametable for now
//         let tile = ppu.ram[i] as u16;
//         let chr_rom_index = (bank + tile * 16) as usize;
//         let tile = &ppu.chr_rom[chr_rom_index..=(chr_rom_index + 15)];
//
//         let tile_x = i % 32;
//         let tile_y = i / 32;
//         for y in 0..=7 {
//             let mut upper = tile[y];
//             let mut lower = tile[y + 8];
//
//             for x in (0..=7).rev() {
//                 let value = (1 & upper) << 1 | (1 & lower);
//                 upper >>= 1;
//                 lower >>= 1;
//                 let color = match value {
//                     0 => COLOR_MAP.get_color(0x01),
//                     1 => COLOR_MAP.get_color(0x23),
//                     2 => COLOR_MAP.get_color(0x27),
//                     3 => COLOR_MAP.get_color(0x30),
//                     _ => panic!("Impossible color index"),
//                 };
//                 // let color = COLOR_MAP.get_color(0x23);
//                 frame.set_pixel(tile_x*8 + x, tile_y*8 + y, *color)
//             }
//         }
//     }
// }
pub fn render(ppu: &PPU, frame: &mut Frame) {
    let bank = ppu.ctrl_register.background_pattern_addr();

    for i in 0..0x3c0 {
        // Access the tile data from the CHR ROM
        let tile_index = ppu.ram[i] as u16; // Get tile index from the PPU RAM
        let tile_start = (bank + tile_index * 16) as usize;
        let tile = &ppu.chr_rom[tile_start..tile_start + 16];
        // println!("tile: {:?}", tile);

        let tile_column = i % 32; // Calculate the column of the nametable
        let tile_row = i / 32; // Calculate row in the nametable

        for y in 0..8 {
            let upper = tile[y]; // Upper byte of the tile
            let lower = tile[y + 8]; // Lower byte of the tile

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