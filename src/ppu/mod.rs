mod address_register;
mod control_register;
mod mask_register;
mod scroll_register;
mod status_register;

use crate::cartridge::Cartridge;
use crate::ppu::control_register::ControlRegister;
use crate::ppu::mask_register::MaskRegister;
use crate::ppu::scroll_register::ScrollRegister;
use crate::ppu::status_register::StatusRegister;
use crate::rom::Mirroring;

const OAM_SIZE: usize = 256;
const RAM_SIZE: usize = 2048;
const NAME_TABLE_SIZE: usize = 0x400; // Size of each nametable (1 KB)
const PALETTE_SIZE: usize = 0x20; // Size of the palette memory

pub trait PpuBusInterface {
    fn chr_read(&mut self, addr: u16) -> u8;
    fn chr_write(&mut self, addr: u16, value: u8);
    fn mirroring(&mut self) -> Mirroring;
    fn nmi(&mut self);
}

pub struct PPU {
    bus: Option<*mut dyn PpuBusInterface>,
    pub palette_table: [u8; PALETTE_SIZE],
    pub ram: [u8; RAM_SIZE], // $2007 (R - latched)

    pub ctrl_register: ControlRegister,  // $2000 (W)
    pub mask_register: MaskRegister,     // $2001 (w)
    pub status_register: StatusRegister, // $2002 (R)
    pub oam_addr: u8,                    // $2003 (W)
    pub oam_data: [u8; 256],             // $2004 (R/W) Object Attribute Memory
    pub scroll_register: ScrollRegister, // $2005 / $2006 - (write latched)

    internal_data: u8,
    pub cycles: usize,
    scanline: usize,
    pub frame_buffer: [u8; 256 * 240],
    pub nmi_interrupt: Option<u8>,
}

impl PPU {
    pub fn new() -> Self {
        PPU {
            bus: None,
            ram: [0; RAM_SIZE],
            oam_addr: 0,
            oam_data: [0; OAM_SIZE],
            palette_table: [0; 32],
            // addr_register: AddressRegister::new(),
            ctrl_register: ControlRegister::from_bits_truncate(0b0),
            mask_register: MaskRegister::new(),
            scroll_register: ScrollRegister::new(),
            status_register: StatusRegister::new(),
            internal_data: 0,
            cycles: 0,
            scanline: 0,
            frame_buffer: [0u8; 256 * 240],
            nmi_interrupt: None,
        }
    }

    /// `connect_bus` MUST be called after constructing PPU
    pub fn connect_bus(&mut self, bus: *mut dyn PpuBusInterface) {
        self.bus = Some(bus);
    }

    pub fn read_register(&mut self, addr: u16) -> u8 {
        match addr & 0x2007 {
            0x2002 => self.read_status(),
            0x2004 => self.read_oam_data(),
            0x2007 => self.read_memory(),
            _ => {
                println!("Unhandled PPU read at: {addr:04X}");
                0
            }
        }
    }

    // pub fn write_register(&mut self, addr: u16, value: u8) {
    //     match addr & 0x2007 {
    //         0x2000 => self.write_to_ctrl(value),
    //         0x2001 => self.write_to_mask(value),
    //         0x2003 => self.set_oam_addr(value),
    //         0x2004 => self.write_to_oam_data(value),
    //         0x2005 => self.write_to_scroll(value),
    //         0x2006 => self.write_to_addr(value),
    //         0x2007 => self.write_memory(value),
    //         _ => println!("Unhandled PPU write at: {addr:04X} = {value:02X}"),
    //     }
    // }
    pub fn write_register(&mut self, addr: u16, value: u8) {
        match addr {
            0x2000 => self.write_to_ctrl(value),
            0x2001 => self.write_to_mask(value),
            0x2003 => self.set_oam_addr(value),
            0x2004 => self.write_to_oam_data(value),
            0x2005 => self.write_to_scroll(value),
            0x2006 => self.write_to_addr(value),
            0x2007 => self.write_memory(value),
            0x2008..=0x3FFF => {
                // Mirror $2000–$2007 every 8 bytes
                self.write_register(0x2000 + (addr % 8), value);
            }
            _ => println!("Unhandled PPU write at: {addr:04X} = {value:02X}"),
        }
    }

    pub fn tick(&mut self) -> bool {
        // Visible scanlines (0–239)
        if self.scanline < 240 {
            match self.cycles {
                257 => {
                    self.render_scanline();
                    self.scroll_register.copy_horizontal_bits();
                }
                _ => {}
            }
        }

        // Pre-render scanline (261)
        if self.scanline == 261 {
            match self.cycles {
                1 => self.scroll_register.copy_vertical_bits(),
                _ => {}
            }
        }

        self.cycles += 1;

        if self.cycles >= 341 {
            self.cycles = 0;
            self.scanline += 1;

            // Only increment_y on visible scanlines and pre-render (scanline 261)
            if self.scanline < 240 || self.scanline == 261 {
                self.scroll_register.increment_y();
            }

            if self.scanline == 241 {
                self.status_register.set_vblank_status(true);

                if self.ctrl_register.generate_vblank_nmi() {
                    if let Some(bus_ptr) = self.bus {
                        unsafe {
                            (*bus_ptr).nmi();
                        }
                    }
                }
            } else if self.scanline == 261 {
                self.status_register.reset_vblank_status();
            }

            if self.scanline >= 262 {
                self.scanline = 0;
                return true; // Frame ready
            }
        }

        false
    }

    // pub fn tick(&mut self) -> bool {
    //     // HBLANK starts at cycle 257
    //     if self.scanline < 240 && self.cycles == 257 {
    //         self.render_scanline();
    //     }
    //
    //     self.cycles += 1;
    //     if self.cycles >= 341 {
    //         self.cycles -= 341;
    //         self.scanline += 1;
    //
    //         // At the end of the scanline, increment vertical scroll
    //         self.increment_y();
    //
    //         if self.scanline == 241 && self.cycles == 0 {
    //             self.status_register.set_vblank_status(true);
    //
    //             if self.ctrl_register.generate_vblank_nmi() {
    //                 if let Some(bus_ptr) = self.bus {
    //                     unsafe {
    //                         (*bus_ptr).nmi();
    //                     }
    //                 }
    //             }
    //         } else if self.scanline == 261 && self.cycles == 0 {
    //             self.status_register.reset_vblank_status();
    //         }
    //
    //         if self.scanline >= 262 {
    //             self.scanline = 0;
    //             // panic!("self.scanline = {}", self.scanline);
    //             return true; // Frame ready to draw
    //         }
    //     }
    //     false
    // }

    // pub fn read_bytes_raw(&mut self, address: usize, size: usize) -> Vec<u8> {
    //     self.ram[address..=address + size].to_vec()
    // }
}

// Private implementations
impl PPU {
    // #[inline]
    // fn render_scanline(&mut self) {
    //     let y = self.scanline;
    //     for x in 0..256 {
    //         let bg_color = self.get_background_pixel(x, y);
    //         let sprite = self.get_sprite_pixel(x, y);
    //
    //         let final_color = match sprite {
    //             Some((sprite_color, is_sprite_zero, in_front)) => {
    //                 if is_sprite_zero && bg_color != self.palette_table[0] {
    //                     self.status_register.set_sprite_zero_hit(true);
    //                 }
    //
    //                 if in_front || bg_color == self.palette_table[0] {
    //                     sprite_color
    //                 } else {
    //                     bg_color
    //                 }
    //             }
    //             None => bg_color,
    //         };
    //
    //         self.frame_buffer[y * 256 + x] = final_color;
    //     }
    // }

    #[inline]
    fn render_scanline(&mut self) {
        let scanline = self.scanline;

        // At dot 257: copy horizontal bits from t to v
        if self.mask_register.show_background() || self.mask_register.show_sprites() {
            self.scroll_register.v = (self.scroll_register.v & 0b111_1011_1110_0000)
                | (self.scroll_register.t & 0b000_0100_0001_1111);
        }

        let mut v = self.scroll_register.v;
        let fine_x = self.scroll_register.x;

        for i in 0..256 {
            let x_offset = (i + fine_x as usize) & 0x1FF; // wrap at 512 just in case
            let bg_color = self.get_background_pixel(v, (x_offset % 8) as u8); // fine_x within tile

            let sprite = self.get_sprite_pixel(i, scanline);

            let final_color = match sprite {
                Some((sprite_color, is_sprite_zero, in_front)) => {
                    if is_sprite_zero && bg_color != self.palette_table[0] {
                        self.status_register.set_sprite_zero_hit(true);
                    }

                    if in_front || bg_color == self.palette_table[0] {
                        sprite_color
                    } else {
                        bg_color
                    }
                }
                None => bg_color,
            };

            self.frame_buffer[scanline * 256 + i] = final_color;

            // Increment v every 8 pixels (end of tile)
            if (i + 1) % 8 == 0 {
                self.scroll_register.increment_x();
                v = self.scroll_register.v; // update local v from scroll register
            }
        }

        // self.increment_y(); // Do this once per scanline
    }

    // #[inline]
    // fn render_scanline(&mut self) {
    //     let scanline = self.scanline;
    //
    //     // Copy horizontal scroll bits from t to v at the start of visible scanline
    //     if self.mask_register.show_background() || self.mask_register.show_sprites() {
    //         self.scroll_register.v = (self.scroll_register.v & 0b111_1011_1110_0000) | (self.scroll_register.t & 0b000_0100_0001_1111);
    //     }
    //
    //     let mut v = self.scroll_register.v;
    //     let fine_x = self.scroll_register.x;
    //
    //     for x in 0..256 {
    //         let bg_color = self.get_background_pixel(v, fine_x);
    //
    //         let sprite = self.get_sprite_pixel(x, scanline);
    //
    //         let final_color = match sprite {
    //             Some((sprite_color, is_sprite_zero, in_front)) => {
    //                 if is_sprite_zero && bg_color != self.palette_table[0] {
    //                     self.status_register.set_sprite_zero_hit(true);
    //                 }
    //
    //                 if in_front || bg_color == self.palette_table[0] {
    //                     sprite_color
    //                 } else {
    //                     bg_color
    //                 }
    //             }
    //             None => bg_color,
    //         };
    //
    //         self.frame_buffer[scanline * 256 + x] = final_color;
    //
    //         // Increment coarse X every pixel, handle nametable switch at 32
    //         if (v & 0x001F) == 31 {
    //             v &= !0x001F;
    //             v ^= 0x0400; // switch horizontal nametable
    //         } else {
    //             v += 1;
    //         }
    //     }
    //
    //     self.scroll_register.v = v;
    //
    // }

    #[inline]
    fn get_background_pixel(&mut self, v: u16, fine_x: u8) -> u8 {
        let coarse_x = v & 0b11111;
        let coarse_y = (v >> 5) & 0b11111;
        let nametable_index = (v >> 10) & 0b11;
        let fine_y = (v >> 12) & 0b111;

        let nametable_addr = 0x2000 | ((nametable_index << 10) + (coarse_y * 32) + coarse_x) as u16;
        let tile_index = self.ram[self.mirror_ram_addr(nametable_addr) as usize];

        let pattern_addr = self.ctrl_register.background_pattern_addr() + (tile_index as u16) * 16 + fine_y;
        let low_plane = self.chr_read(pattern_addr);
        let high_plane = self.chr_read(pattern_addr + 8);

        let shift = 7 - fine_x;
        let low_bit = (low_plane >> shift) & 1;
        let high_bit = (high_plane >> shift) & 1;
        let color_index = (high_bit << 1) | low_bit;

        if color_index == 0 {
            return self.palette_table[0]; // universal background color
        }

        // Attribute table calculation
        let attribute_addr = 0x23C0 | ((nametable_index & 0b11) << 10)
            | ((coarse_y / 4) << 3)
            | (coarse_x / 4);
        let attribute_byte = self.ram[self.mirror_ram_addr(attribute_addr) as usize];

        let quadrant = ((coarse_y % 4) / 2) * 2 + ((coarse_x % 4) / 2);
        let palette_bits = (attribute_byte >> (quadrant * 2)) & 0b11;

        let palette_addr = 0x3F00 + (palette_bits as u16) * 4 + (color_index as u16);
        self.palette_table[(palette_addr & 0x1F) as usize]
    }


    // #[inline]
    // fn get_background_pixel(&mut self, x: usize, y: usize) -> u8 {
    //     let coarse_x = x / 8;
    //     let coarse_y = y / 8;
    //     let fine_x = x % 8;
    //     let fine_y = y % 8;
    //
    //     let nametable_base = self.ctrl_register.get_nametable_addr() as usize;
    //     let tile_index = self.ram[self.mirror_ram_addr((nametable_base + coarse_y * 32 + coarse_x) as u16) as usize];
    //
    //     let pattern_table_addr = self.ctrl_register.background_pattern_addr();
    //     let tile_addr = pattern_table_addr + (tile_index as u16) * 16 + fine_y as u16;
    //
    //     let low_plane = unsafe { (*self.bus.unwrap()).chr_read(tile_addr) };
    //     let high_plane = unsafe { (*self.bus.unwrap()).chr_read(tile_addr + 8) };
    //
    //     let bit = 7 - fine_x;
    //     let color_index = ((high_plane >> bit) & 1) << 1 | ((low_plane >> bit) & 1);
    //
    //     if color_index == 0 {
    //         return self.palette_table[0];
    //     }
    //
    //     let attribute_addr = nametable_base + 0x3C0 + (coarse_y / 4) * 8 + (coarse_x / 4);
    //     let attribute_byte = self.ram[self.mirror_ram_addr(attribute_addr as u16) as usize];
    //
    //     let shift = {
    //         let quadrant_x = (coarse_x % 4) / 2;
    //         let quadrant_y = (coarse_y % 4) / 2;
    //         (quadrant_y * 2 + quadrant_x) * 2
    //     };
    //
    //     let palette_select = (attribute_byte >> shift) & 0b11;
    //     let palette_addr = 0x3F00 + (palette_select as u16) * 4 + (color_index as u16);
    //
    //     self.palette_table[(palette_addr & 0x1F) as usize]
    // }

    /// Returns (palette index, is_sprite_zero)
    fn get_sprite_pixel(&mut self, x: usize, y: usize) -> Option<(u8, bool, bool)> {
        let sprite_height = self.ctrl_register.sprite_size() as usize;

        for i in 0..64 {
            let base = i * 4;
            let sprite_y = self.oam_data[base] as usize;
            let tile_index = self.oam_data[base + 1];
            let attributes = self.oam_data[base + 2];
            let sprite_x = self.oam_data[base + 3] as usize;

            // Check if the scanline intersects the sprite vertically
            if y < sprite_y || y >= sprite_y + sprite_height {
                continue;
            }

            // Check horizontal bounds
            if x < sprite_x || x >= sprite_x + 8 {
                continue;
            }

            let flip_vertical = attributes & 0x80 != 0;
            let flip_horizontal = attributes & 0x40 != 0;
            let palette_select = attributes & 0x03;
            let priority = (attributes & 0x20) == 0;

            let fine_y = if flip_vertical {
                sprite_height - 1 - (y - sprite_y)
            } else {
                y - sprite_y
            };

            let fine_x = if flip_horizontal {
                7 - (x - sprite_x)
            } else {
                x - sprite_x
            };

            let pattern_addr = if sprite_height == 16 {
                // 8x16 sprite mode
                let table = tile_index & 0x01;
                let index = tile_index & 0xFE;
                let base = (table as u16) << 12;
                base + (index as u16) * 16 + (fine_y as u16)
            } else {
                // 8x8 sprite mode
                let base = self.ctrl_register.sprite_pattern_addr();
                base + (tile_index as u16) * 16 + (fine_y as u16)
            };

            let low = self.chr_read(pattern_addr);
            let high = self.chr_read(pattern_addr + 8);

            let bit = 7 - fine_x;
            let low_bit = (low >> bit) & 1;
            let high_bit = (high >> bit) & 1;
            let color_index = (high_bit << 1) | low_bit;

            if color_index == 0 {
                continue; // Transparent pixel
            }

            let palette_addr = 0x3F10 + (palette_select as u16) * 4 + (color_index as u16);
            let palette_index = self.palette_table[(palette_addr & 0x1F) as usize];

            return Some((palette_index, i == 0, priority));
        }
        None
    }

    fn chr_read(&mut self, addr: u16) -> u8 {
        match self.bus {
            Some(bus_ptr) => unsafe { (*bus_ptr).chr_read(addr) },
            None => {
                eprintln!("Invalid PPU::chr_read at address: {:04X}", addr);
                0
            }
        }
    }

    fn chr_write(&mut self, addr: u16, value: u8) {
        match self.bus {
            Some(bus_ptr) => unsafe { (*bus_ptr).chr_write(addr, value) },
            None => {
                eprintln!("Invalid PPU::chr_write at address: {:04X}", addr);
            }
        }
    }

    fn mirroring(&self) -> Mirroring {
        match self.bus {
            Some(bus_ptr) => unsafe { (*bus_ptr).mirroring() },
            None => {
                eprintln!("Unable to detect Cartridge mirroring mode");
                Mirroring::Vertical
            }
        }
    }

    fn read_memory(&mut self) -> u8 {
        // let addr = self.addr_register.get() & 0x3FFF;
        let addr = self.scroll_register.get_addr();

        let result = match addr {
            0..=0x1FFF => self.chr_read(addr),
            0x2000..=0x2FFF => {
                let result = self.internal_data;
                self.internal_data = self.ram[self.mirror_ram_addr(addr) as usize];
                result
            }
            0x3000..=0x3EFF => {
                let result = self.internal_data;
                self.internal_data = self.ram[self.mirror_ram_addr(addr) as usize];
                result
            }
            0x3F00..=0x3FFF => {
                // Palette RAM (32 bytes mirrored every $20)
                let mirrored_addr = (addr - 0x3F00) % 0x20;
                self.palette_table[mirrored_addr as usize]
            }
            _ => {
                eprintln!("Unhandled PPU::read_memory() at {:04X}", addr);
                0
            }
        };

        self.increment_addr();
        result
    }

    fn write_memory(&mut self, value: u8) {
        // let addr = self.addr_register.get() & 0x3FFF;
        let addr = self.scroll_register.get_addr();
        // println!("ppu::write_memory(${:04X}) = ${:02X}", addr, value);

        match addr {
            0x0000..=0x1FFF => {
                self.chr_write(addr, value);
            }

            0x2000..=0x2FFF | 0x3000..=0x3EFF => {
                let mirrored = self.mirror_ram_addr(addr);
                self.ram[mirrored as usize] = value;
            }

            0x3F00..=0x3FFF => {
                let mut palette_addr = (addr - 0x3F00) % 0x20;

                // Handle mirrors of universal background color
                match palette_addr {
                    0x10 | 0x14 | 0x18 | 0x1C => palette_addr -= 0x10,
                    _ => {}
                }

                self.palette_table[palette_addr as usize] = value;
            }

            _ => {
                eprintln!("Unhandled PPU::write_memory() at {:04X}", addr);
            }
        }

        self.increment_addr();
    }
}

// Helpers
impl PPU {
    fn read_status(&mut self) -> u8 {
        let data = self.status_register.value();
        self.status_register.reset_vblank_status();
        self.scroll_register.reset_latch();
        data
    }

    fn get_nmi_status(&mut self) -> Option<u8> {
        self.nmi_interrupt.take()
    }

    fn set_oam_addr(&mut self, value: u8) {
        self.oam_addr = value;
    }

    fn read_oam_data(&self) -> u8 {
        self.oam_data[self.oam_addr as usize]
    }

    fn write_to_oam_data(&mut self, value: u8) {
        self.oam_data[self.oam_addr as usize] = value;
        self.oam_addr = self.oam_addr.wrapping_add(1);
    }

    fn write_to_oam_dma(&mut self, data: &[u8; 256]) {
        for x in data.iter() {
            self.oam_data[self.oam_addr as usize] = *x;
            self.oam_addr = self.oam_addr.wrapping_add(1);
        }
    }

    fn write_to_scroll(&mut self, value: u8) {
        self.scroll_register.write(value);
    }

    fn write_to_addr(&mut self, value: u8) {
        // println!("ppu.set_ppu_addr(${:02X})", value);
        // self.addr_register.update(value);
        self.scroll_register.write_to_addr(value);
    }
    fn write_to_ctrl(&mut self, value: u8) {
        // Automatically generate NMI if:
        //      - PPU is in VBLANK
        //      - GENERATE_NMI toggles from 0 to 1
        let prev_generate_nmi = self.ctrl_register.generate_vblank_nmi();
        self.ctrl_register.update(value);
        if !prev_generate_nmi
            && self.ctrl_register.generate_vblank_nmi()
            && self.status_register.is_in_vblank()
        {
            self.nmi_interrupt = Some(1);
        }
    }
    fn write_to_mask(&mut self, value: u8) {
        self.mask_register.update(value);
    }

    fn increment_addr(&mut self) {
        self.scroll_register
            .increment_addr(self.ctrl_register.addr_increment());
    }

    fn mirror_ram_addr(&self, addr: u16) -> u16 {
        let mirrored_addr = addr & 0x2FFF;
        let index = mirrored_addr - 0x2000;

        let table = index / 0x400;
        let offset = index % 0x400;

        match self.mirroring() {
            Mirroring::Vertical => {
                // NT0 and NT2 share, NT1 and NT3 share
                match table {
                    0 | 2 => offset,           // NT0 or NT2
                    1 | 3 => offset + 0x400,   // NT1 or NT3
                    _ => unreachable!(),
                }
            }
            Mirroring::Horizontal => {
                // NT0 and NT1 share, NT2 and NT3 share
                match table {
                    0 | 1 => offset,           // NT0 or NT1
                    2 | 3 => offset + 0x400,   // NT2 or NT3
                    _ => unreachable!(),
                }
            }
            Mirroring::FourScreen => {
                // If you actually support 4-screen (most emulators don’t),
                // you'd need a separate 4KB RAM buffer — for now, mirror as-is:
                index % 0x800
            }
            Mirroring::Single0 => {
                // Always map to $2000 (NT0)
                offset
            }
            Mirroring::Single1 => {
                // Always map to $2400 (NT1)
                offset + 0x400
            }
            _ => unimplemented!()
        }
    }
    //
    // fn increment_x(&mut self) {
    //     if (self.scroll_register.v & 0x001F) == 31 {
    //         self.scroll_register.v &= !0x001F;
    //         self.scroll_register.v ^= 0x0400;
    //     } else {
    //         self.scroll_register.v += 1;
    //     }
    // }
    //
    // fn increment_y(&mut self) {
    //     let mut v = self.scroll_register.v;
    //     if (v & 0x7000) != 0x7000 {
    //         v += 0x1000; // increment fine Y
    //     } else {
    //         v &= !0x7000; // fine Y = 0
    //         let mut y = (v >> 5) & 0x1F;
    //         if y == 29 {
    //             y = 0;
    //             v ^= 0x0800; // switch vertical nametable
    //         } else if y == 31 {
    //             y = 0;
    //         } else {
    //             y += 1;
    //         }
    //         v = (v & !0x03E0) | (y << 5);
    //     }
    //     self.scroll_register.v = v;
    // }
    //
    // fn copy_horizontal_bits(&mut self) {
    //     // v: .....F.. ...EDCBA = t: .....F.. ...EDCBA
    //     // Copy NT X and coarse X (bits 10 and 0-4)
    //     self.scroll_register.v &= !0b0000010000011111;
    //     self.scroll_register.v |= self.scroll_register.t & 0b0000010000011111;
    // }
    //
    // fn copy_vertical_bits(&mut self) {
    //     // v: .IHGF.ED CBA..... = t: .IHGF.ED CBA.....
    //     // Copy fine Y, coarse Y, and NT Y (bits 12-5 and bit 11)
    //     self.scroll_register.v &= !0b0111101111100000;
    //     self.scroll_register.v |= self.scroll_register.t & 0b0111101111100000;
    // }




    // pub fn get_nametable(&self, nt_x: u16, nt_y: u16) -> &[u8] {
    //     let mirroring = self.cart.borrow().mirroring();
    //
    //     let index = match mirroring {
    //         Mirroring::Vertical => nt_x % 2 + (nt_y % 2) * 2,
    //         Mirroring::Horizontal => (nt_x / 2) % 2 + ((nt_y % 2) * 2),
    //         Mirroring::FourScreen => (nt_x % 2) + (nt_y % 2) * 2,
    //         _ => panic!("Unsupported mirroring: {:?}", mirroring),
    //     };
    //
    //     match index {
    //         0 => &self.ram[0x000..0x400],
    //         1 => &self.ram[0x400..0x800],
    //         2 => &self.ram[0x000..0x400],
    //         3 => &self.ram[0x400..0x800],
    //         _ => unreachable!(),
    //     }
    // }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::bus::simple_bus::SimpleBus;

    struct MockPpuBus {
        pub chr: [u8; 0x2000],
        pub triggered_nmi: bool,
        pub mirroring: Mirroring,
    }

    impl MockPpuBus {
        fn new() -> Self {
            Self {
                chr: [0; 0x2000],
                triggered_nmi: false,
                mirroring: Mirroring::Horizontal,
            }
        }
    }
    impl PpuBusInterface for MockPpuBus {
        fn chr_read(&mut self, addr: u16) -> u8 {
            self.chr[addr as usize % 0x2000]
        }
        fn chr_write(&mut self, addr: u16, value: u8) {
            self.chr[addr as usize % 0x2000] = value;
        }
        fn mirroring(&mut self) -> Mirroring {
            self.mirroring.clone()
        }
        fn nmi(&mut self) {
            self.triggered_nmi = true;
        }
    }

    fn init_mock_ppu() -> PPU {
        let mut ppu = PPU::new();
        let mut mock_bus = MockPpuBus::new();
        ppu.connect_bus(&mut mock_bus as *mut _);
        ppu
    }

    #[test]
    fn test_write_memory_via_registers() {
        let mut ppu = init_mock_ppu();
        let want = 0x42;

        // Set address via $2006 (high byte first, then low byte)
        ppu.write_register(0x2006, 0x21); // 0x2100 - Nametable 1
        ppu.write_register(0x2006, 0x00);

        // Write to memory via $2007
        ppu.write_register(0x2007, want);

        // Assuming horizontal mirroring: 0x2100 maps to 0x0100 in internal VRAM
        let mirrored = ppu.mirror_ram_addr(0x2100) as usize;
        assert_eq!(mirrored, 0x0100);

        // Verify VRAM
        let got = ppu.ram[mirrored];
        assert_eq!(got, want);
    }

    #[test]
    fn test_scroll_register_horizontal_and_vertical_write() {
        let mut ppu = init_mock_ppu();

        // First write to $2005 sets coarse X and fine X
        ppu.write_register(0x2005, 0b00110101); // value = 0x35

        assert_eq!(ppu.scroll_register.w, true);
        assert_eq!(ppu.scroll_register.t & 0b00000_11111, 6); // coarse X = 6
        assert_eq!(ppu.scroll_register.x, 0b101); // fine X = 5

        // Second write sets coarse Y and fine Y
        ppu.write_register(0x2005, 0b11010111); // 0xD7

        assert_eq!(ppu.scroll_register.w, false);
        assert_eq!((ppu.scroll_register.t >> 5) & 0b11111, 0b11010); // coarse Y = 26
        assert_eq!((ppu.scroll_register.t >> 12) & 0b111, 0b111); // fine Y = 7
    }

    #[test]
    fn test_write_to_2006_sets_t_and_v() {
        let mut ppu = init_mock_ppu();

        ppu.write_register(0x2006, 0x3F); // High byte of address
        assert_eq!(ppu.scroll_register.t, 0x3F00);
        assert_eq!(ppu.scroll_register.w, true);

        ppu.write_register(0x2006, 0x10); // Low byte
        assert_eq!(ppu.scroll_register.t, 0x3F10);
        assert_eq!(ppu.scroll_register.v, 0x3F10);
        assert_eq!(ppu.scroll_register.w, false);
    }

    #[test]
    fn test_increment_y_behavior() {
        let mut ppu = init_mock_ppu();

        // Set fine Y to 7, so it will overflow
        ppu.scroll_register.v = 0;
        ppu.scroll_register.v |= 7 << 12; // fine Y = 7
        ppu.scroll_register.v |= 5 << 5;  // coarse Y = 5
        ppu.scroll_register.increment_y();

        // Should reset fine Y to 0 and increment coarse Y
        assert_eq!((ppu.scroll_register.v >> 12) & 0b111, 0); // fine Y
        assert_eq!((ppu.scroll_register.v >> 5) & 0b11111, 6); // coarse Y
    }
}
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::cartridge::nrom::NromCart;
//
//     fn create_empty_ppu() -> PPU {
//         let cart = NromCart::new(vec![0; 0x4000], vec![0; 0x4000], Mirroring::Vertical);
//         PPU::new(Rc::new(RefCell::new(cart)))
//     }
//
//     #[test]
//     fn test_write_chr_rom_space() {
//         let mut ppu = create_empty_ppu();
//         ppu.write_to_data(0x55);
//         // Expecting no change in CHR ROM since it's read-only (unless CHR RAM is supported)
//         assert_ne!(ppu.cart.borrow_mut().chr_read(0), 0x55);
//     }
//
//     #[test]
//     fn test_write_nametable_ram() {
//         let mut ppu = create_empty_ppu();
//         ppu.addr_register.set(0x2005);
//         ppu.write_to_data(0xAB);
//         assert_eq!(ppu.ram[ppu.mirror_ram_addr(0x2005) as usize], 0xAB);
//     }
//
//     #[test]
//     fn test_write_mirrored_nametable_ram() {
//         let mut ppu = create_empty_ppu();
//         ppu.addr_register.set(0x3005);
//         ppu.write_to_data(0xCD);
//         assert_eq!(ppu.ram[ppu.mirror_ram_addr(0x3005) as usize], 0xCD);
//     }
//
//     #[test]
//     fn test_write_palette_table_direct() {
//         let mut ppu = create_empty_ppu();
//         ppu.addr_register.set(0x3F00);
//         ppu.write_to_data(0x12);
//         assert_eq!(ppu.palette_table[0], 0x12);
//     }
//
//     #[test]
//     fn test_write_palette_table_mirrored() {
//         let mut ppu = create_empty_ppu();
//         ppu.addr_register.set(0x3F10);
//         ppu.write_to_data(0x34);
//         assert_eq!(ppu.palette_table[0], 0x34); // Should be mirrored to 0x3F00
//     }
// }
