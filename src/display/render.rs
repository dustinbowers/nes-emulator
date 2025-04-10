// use crate::cpu::processor::CPU;
// use crate::display::color_map::COLOR_MAP;
// use crate::display::frame::Frame;
// use crate::ppu::PPU;
// use crate::rom::Mirroring;
// use macroquad::prelude::draw_rectangle;
// use std::cell::RefCell;
// use std::rc::Rc;
//
// struct ViewPort {
//     x1: usize,
//     y1: usize,
//     x2: usize,
//     y2: usize,
// }
// impl ViewPort {
//     pub fn new(x1: usize, y1: usize, x2: usize, y2: usize) -> Self {
//         Self { x1, y1, x2, y2 }
//     }
// }
//
// // pub fn render(ppu: &PPU, mut frame: Rc<RefCell<Frame>>) {
// //
// //
// //     // let (main_nametable, second_nametable) = match ppu.cart.borrow().mirroring() {
// //     //     Mirroring::Vertical => match ppu.ctrl_register.get_nametable_addr() {
// //     //         0x2000 | 0x2800 => (&ppu.ram[0..0x400], &ppu.ram[0x400..0x800]),
// //     //         0x2400 | 0x2C00 => (&ppu.ram[0x400..0x800], &ppu.ram[0..0x400]),
// //     //         _ => panic!(
// //     //             "Unexpected nametable address: {:04X}",
// //     //             ppu.ctrl_register.get_nametable_addr()
// //     //         ),
// //     //     },
// //     //     Mirroring::Horizontal => match ppu.ctrl_register.get_nametable_addr() {
// //     //         0x2000 | 0x2400 => (&ppu.ram[0..0x400], &ppu.ram[0x400..0x800]),
// //     //         0x2800 | 0x2C00 => (&ppu.ram[0x400..0x800], &ppu.ram[0..0x400]),
// //     //         _ => panic!(
// //     //             "Unexpected nametable address: {:04X}",
// //     //             ppu.ctrl_register.get_nametable_addr()
// //     //         ),
// //     //     },
// //     //     _ => panic!(
// //     //         "Unsupported mirroring type: {:?}",
// //     //         ppu.cart.borrow().mirroring()
// //     //     ),
// //     // };
// //
// //     if ppu.mask_register.show_background() {
// //         // let scroll_x = ppu.scroll_register.scroll_x as u16;
// //         // let scroll_y = ppu.scroll_register.scroll_y as u16;
// //         // let offset_x = scroll_x % 256;
// //         // let offset_y = scroll_y % 240;
// //         // for ty in 0..=1 {
// //         //     for tx in 0..=1 {
// //         //         // which logical nametable to sample
// //         //         let nt_x = (scroll_x / 256 + tx) % 2;
// //         //         let nt_y = (scroll_y / 240 + ty) % 2;
// //         //         let nametable = ppu.get_nametable(nt_x as u16, nt_y as u16);
// //         //
// //         //         // source viewport in that nametable
// //         //         let src_x = if tx == 0 { offset_x } else { 0 };
// //         //         let src_y = if ty == 0 { offset_y } else { 0 };
// //         //         let width  = if tx == 0 { 256 - offset_x } else { offset_x };
// //         //         let height = if ty == 0 { 240 - offset_y } else { offset_y };
// //         //
// //         //         // where on screen to draw it
// //         //         let dest_x = (tx as isize * 256) - offset_x as isize;
// //         //         let dest_y = (ty as isize * 240) - offset_y as isize;
// //         //
// //         //         // only draw if there’s something to draw
// //         //         if width > 0 && height > 0 {
// //         //             render_name_table(
// //         //                 ppu,
// //         //                 &mut frame,
// //         //                 nametable,
// //         //                 ViewPort::new(
// //         //                     src_x as usize,
// //         //                     src_y as usize,
// //         //                     width as usize,
// //         //                     height as usize),
// //         //                 dest_x,
// //         //                 dest_y,
// //         //             );
// //         //         }
// //         //     }
// //         // }
// //         render_background(ppu, &mut frame);
// //     }
// //
// //     /// //////////////////////////
// //
// //
// //     // // Render background nametable
// //     // let scroll_x = ppu.scroll_register.scroll_x as usize;
// //     // let scroll_y = ppu.scroll_register.scroll_y as usize;
// //     // render_name_table(
// //     //     ppu,
// //     //     &mut frame,
// //     //     main_nametable,
// //     //     ViewPort::new(scroll_x, scroll_y, 256, 240),
// //     //     -(scroll_x as isize),
// //     //     -(scroll_y as isize),
// //     // );
// //     //
// //     // if scroll_x > 0 {
// //     //     render_name_table(
// //     //         ppu,
// //     //         &mut frame,
// //     //         second_nametable,
// //     //         ViewPort::new(0, 0, scroll_x, 240),
// //     //         (256 - scroll_x) as isize,
// //     //         0,
// //     //     );
// //     // } else if scroll_y > 0 {
// //     //     render_name_table(
// //     //         ppu,
// //     //         &mut frame,
// //     //         second_nametable,
// //     //         ViewPort::new(0, 0, 256, 240),
// //     //         0,
// //     //         (240 - scroll_y) as isize,
// //     //     );
// //     // }
// //
// //     if ppu.mask_register.show_sprites() == false {
// //         return;
// //     }
// //
// //     // Render sprites
// //     for i in (0..ppu.oam_data.len()).step_by(4).rev() {
// //         let tile_y = ppu.oam_data[i + 0] as usize;
// //         let tile_index = ppu.oam_data[i + 1] as u16;
// //         let tile_attributes = ppu.oam_data[i + 2];
// //         let tile_x = ppu.oam_data[i + 3] as usize;
// //
// //         let flip_vertical = tile_attributes >> 7 & 1 == 1;
// //         let flip_horizontal = tile_attributes >> 6 & 1 == 1;
// //         let palette_index = tile_attributes & 0b11;
// //         let sprite_palette = get_sprite_palette(ppu, palette_index);
// //
// //         let is_8x16 = ppu.ctrl_register.sprite_size() == 16;
// //         let is_second_tile = 1 - (i % 2);
// //         let bank_addr = if is_8x16 {
// //             (tile_index & 1) * 0x1000 // Select correct CHR-ROM bank
// //                                       // unimplemented!("is_8x16 sprite!");
// //         } else {
// //             ppu.ctrl_register.sprite_pattern_addr()
// //         } as usize;
// //         let mut tile_chr_index = (bank_addr + (tile_index as usize * 16)) as u16;
// //         // let tile = &ppu.chr_rom[tile_chr_index..tile_chr_index + 16];
// //
// //         let tile_y = match (is_8x16, is_second_tile) {
// //             (false, _) => tile_y,
// //             (true, 0) => tile_y,
// //             (true, 1) => tile_y + 8,
// //             (_, _) => {
// //                 panic!("impossible");
// //             }
// //         };
// //
// //         let tile_y = if !is_8x16 {
// //             tile_y
// //         } else {
// //             match (flip_vertical, is_second_tile) {
// //                 (false, 0) => tile_y,
// //                 (false, 1) => tile_y + 8,
// //                 (true, 0) => tile_y - 8,
// //                 (true, 1) => tile_y + 8,
// //                 (_, _) => {
// //                     panic!("impossible");
// //                 }
// //             }
// //         };
// //
// //         for y in 0..8 {
// //             // let upper = tile[y];
// //             // let lower = tile[y + 8];
// //             let upper = ppu.cart.borrow_mut().chr_read(tile_chr_index + y);
// //             let lower = ppu.cart.borrow_mut().chr_read(tile_chr_index + y + 8);
// //             for x in 0..8 {
// //                 let pixel_x = if flip_horizontal { 7 - x } else { x };
// //                 let pixel_y = if flip_vertical { 7 - y } else { y } as usize;
// //
// //                 // let pixel_y = match (is_8x16, is_second_tile) {
// //                 //     (false, 0) => { pixel_y }
// //                 //     (false, 1) => { pixel_y + 8 }
// //                 //     (true, 0) => { pixel_y + 8 }
// //                 //     (true, 1) => { pixel_y - 8 }
// //                 //     (_, _) => {}
// //                 // }
// //
// //                 let msb = (lower >> (7 - x)) & 1;
// //                 let lsb = (upper >> (7 - x)) & 1;
// //                 let palette_idx = (msb << 1) | lsb;
// //                 if palette_idx == 0 {
// //                     continue;
// //                 }
// //
// //                 let color = if ppu.mask_register.show_sprites() {
// //                     let mut color_index = sprite_palette[palette_idx as usize] as usize;
// //                     if ppu.mask_register.is_grayscale() {
// //                         color_index &= 0x30
// //                     }
// //                     COLOR_MAP.get_color(color_index)
// //                 } else {
// //                     continue;
// //                 };
// //
// //                 let screen_x = tile_x + pixel_x;
// //                 let screen_y = tile_y + pixel_y;
// //                 frame.borrow_mut().set_pixel(screen_x, screen_y, *color);
// //             }
// //         }
// //     }
// // }
//
// // pub fn render_background(ppu: &PPU, frame: &mut Rc<RefCell<Frame>>) {
// //     let scroll_x = ppu.scroll_register.scroll_x as usize;
// //     let scroll_y = ppu.scroll_register.scroll_y as usize;
// //
// //     let mut y = 0;
// //     let mut src_y = scroll_y % 240;
// //     // let mut nt_y = (scroll_y / 240) % 2;
// //     let mut nt_y = (y / 240) % 2;
// //
// //     while y < 240 {
// //         let draw_height = (240 - src_y).min(240 - y);
// //
// //         let mut x = 0;
// //         let mut src_x = scroll_x % 256;
// //         // let mut nt_x = (scroll_x / 256) % 2;
// //         let mut nt_x = (x / 256) % 2;
// //
// //         while x < 256 {
// //             let draw_width = (256 - src_x).min(256 - x);
// //
// //             let nametable = ppu.get_nametable(nt_x as u16, nt_y as u16);
// //
// //             render_name_table(
// //                 ppu,
// //                 frame,
// //                 nametable,
// //                 ViewPort::new(src_x, src_y, draw_width, draw_height),
// //                 x as isize,
// //                 y as isize,
// //             );
// //
// //             x += draw_width;
// //             src_x = 0;
// //             nt_x = (nt_x + 1) % 2;
// //         }
// //
// //         y += draw_height;
// //         src_y = 0;
// //         nt_y = (nt_y + 1) % 2;
// //     }
// // }
//
// fn render_name_table(
//     ppu: &PPU,
//     frame: &mut Rc<RefCell<Frame>>,
//     name_table: &[u8],
//     view_port: ViewPort,
//     offset_x: isize,
//     offset_y: isize,
// ) {
//     if ppu.mask_register.show_background() == false {
//         return;
//     }
//
//     // Render Background tiles
//     let bank = ppu.ctrl_register.background_pattern_addr();
//     let attribute_table = &name_table[0x3c0..0x400];
//
//     for i in 0..0x3c0 {
//         let tile_index = ppu.ram[i] as u16;
//         let tile_start = bank + tile_index * 16;
//
//         let tile_column = i % 32;
//         let tile_row = i / 32;
//         let palette = get_bg_palette(ppu, attribute_table, tile_column, tile_row);
//
//         for y in 0..8 {
//             let upper = ppu.cart.borrow_mut().chr_read(tile_start + y);
//             let lower = ppu.cart.borrow_mut().chr_read(tile_start + y + 8);
//
//             for x in (0..8).rev() {
//                 let value = ((lower >> (7 - x)) & 1) << 1 | ((upper >> (7 - x)) & 1); // Build pixel value
//
//                 if !ppu.mask_register.show_background() {
//                     continue;
//                 }
//
//                 let mut color_index = match value {
//                     0 => palette[0],
//                     1 => palette[1],
//                     2 => palette[2],
//                     3 => palette[3],
//                     _ => panic!("Impossible color index"),
//                 } as usize;
//
//                 if ppu.mask_register.is_grayscale() {
//                     color_index &= 0x30;
//                 }
//
//                 let color = COLOR_MAP.get_color(color_index);
//                 let px = tile_column * 8 + x;
//                 let py = tile_row * 8 + y as usize;
//                 // frame.set_pixel(tile_column * 8 + x, tile_row * 8 + y, *color)
//                 if px >= view_port.x1
//                     && px <= view_port.x2
//                     && py >= view_port.y1
//                     && py <= view_port.y2
//                 {
//                     let px_shifted = (px as isize + offset_x) as usize;
//                     let py_shifted = (py as isize + offset_y) as usize;
//                     frame.borrow_mut().set_pixel(px_shifted, py_shifted, *color);
//                 }
//             }
//         }
//     }
// }
//
// fn get_bg_palette(
//     ppu: &PPU,
//     attribute_table: &[u8],
//     tile_column: usize,
//     tile_row: usize,
// ) -> [u8; 4] {
//     let attr_table_idx = (tile_row / 4) * 8 + tile_column / 4;
//     let attr_byte = attribute_table[attr_table_idx];
//
//     let pallet_bit_1 = tile_column % 4 / 2;
//     let pallet_bit_0 = tile_row % 4 / 2;
//     let pallet_idx = match (pallet_bit_1, pallet_bit_0) {
//         (0, 0) => (attr_byte >> 0) & 0b11,
//         (1, 0) => (attr_byte >> 2) & 0b11,
//         (0, 1) => (attr_byte >> 4) & 0b11,
//         (1, 1) => (attr_byte >> 6) & 0b11,
//         (_, _) => panic!("impossible pallet_idx"),
//     };
//
//     let palette_start: usize = 0x01 + (pallet_idx as usize) * 4;
//
//     [
//         ppu.palette_table[0],
//         ppu.palette_table[palette_start],
//         ppu.palette_table[palette_start + 1],
//         ppu.palette_table[palette_start + 2],
//     ]
// }
//
// fn get_sprite_palette(ppu: &PPU, palette_idx: u8) -> [u8; 4] {
//     let start = 0x11 + (palette_idx * 4) as usize;
//     [
//         0,
//         ppu.palette_table[start],
//         ppu.palette_table[start + 1],
//         ppu.palette_table[start + 2],
//     ]
// }
//
// #[allow(dead_code)]
// pub fn draw_debug_overlays(cpu: &CPU) {
//     // Debug overlays
//
//     // let ram_px_size = 3;
//     // for (i, v) in cpu.bus.cpu_ram.data.iter().enumerate() {
//     //     let x = i % 32 * ram_px_size + 400;
//     //     let y = i / 32 * ram_px_size + 60;
//     //     draw_rectangle(x as f32, y as f32, ram_px_size as f32, ram_px_size as f32, *COLOR_MAP.get_color((v % 53) as usize));
//     // }
//
//     let ram_px_size = 2;
//     for (i, v) in cpu.bus.ppu.ram.iter().enumerate() {
//         let x = i % 32 * ram_px_size;
//         let y = i / 32 * ram_px_size + 60;
//
//         draw_rectangle(
//             x as f32,
//             y as f32,
//             ram_px_size as f32,
//             ram_px_size as f32,
//             *COLOR_MAP.get_color((v % 53) as usize),
//         );
//     }
//
//     let oam_data_px_size = 2;
//     for (i, v) in cpu.bus.ppu.oam_data.iter().enumerate() {
//         let x = i % 32 * oam_data_px_size;
//         let y = i / 32 * oam_data_px_size + 200;
//         draw_rectangle(
//             x as f32,
//             y as f32,
//             oam_data_px_size as f32,
//             oam_data_px_size as f32,
//             *COLOR_MAP.get_color((v % 53) as usize),
//         );
//     }
//
//     // let chr_data_px_size = 2;
//     // for (i, v) in cpu.bus.ppu.chr_rom.iter().enumerate() {
//     //     let x = i % 64 * chr_data_px_size + 100;
//     //     let y = i / 64 * chr_data_px_size + 40;
//     //     draw_rectangle(x as f32, y as f32, chr_data_px_size as f32, chr_data_px_size as f32, *COLOR_MAP.get_color((v % 53) as usize));
//     // }
//
//     // let prog_rom_px_size = 2;
//     // for (i, v) in cpu.bus.prg_rom.iter().enumerate() {
//     //     let x = i % 64 * prog_rom_px_size + 230;
//     //     let y = i / 64 * prog_rom_px_size + 40;
//     //     draw_rectangle(x as f32, y as f32, prog_rom_px_size as f32, prog_rom_px_size as f32, *COLOR_MAP.get_color((v % 53) as usize));
//     // }
//
//     // let palette_table_px_size = 5;
//     // for (i, v) in cpu.bus.ppu.palette_table.iter().enumerate() {
//     //     let x = i % 32 * palette_table_px_size + 300;
//     //     let y = i / 32 * palette_table_px_size + 32;
//     //     draw_rectangle(x as f32, y as f32, palette_table_px_size as f32, palette_table_px_size as f32, *COLOR_MAP.get_color((v % 53) as usize));
//     // }
//
//     // draw_rectangle(0f32, 0f32, palette_table_px_size as f32, palette_table_px_size as f32, *COLOR_MAP.get_color((cpu.bus.last_fetched_byte % 53) as usize));
// }
