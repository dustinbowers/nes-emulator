mod address_register;
mod control_register;
mod mask_register;
mod mod_tests;
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
    pub(crate) scanline: usize,
    pub frame_buffer: [u8; 256 * 240],
}

impl PPU {
    pub fn new() -> Self {
        PPU {
            bus: None,
            ram: [0; RAM_SIZE],
            oam_addr: 0,
            oam_data: [0; OAM_SIZE],
            palette_table: [0; 32],
            ctrl_register: ControlRegister::from_bits_truncate(0b0),
            mask_register: MaskRegister::new(),
            scroll_register: ScrollRegister::new(),
            status_register: StatusRegister::new(),
            internal_data: 0,
            cycles: 0,
            scanline: 0,
            frame_buffer: [0u8; 256 * 240],
        }
    }

    /// `connect_bus` MUST be called after constructing PPU
    pub fn connect_bus(&mut self, bus: *mut dyn PpuBusInterface) {
        self.bus = Some(bus);
    }

    pub fn read_register(&mut self, addr: u16) -> u8 {
        let reg = addr & 0x2007;
        let result = match reg {
            0x2000..=0x2001 | 0x2003 | 0x2005 | 0x2006 => {
                // write-only, return open-bus or 0
                println!("Read from write-only PPU register: ${:04X}", addr);
                0
            }

            0x2002 => self.read_status(), // PPUSTATUS
            0x2004 => self.oam_data[self.oam_addr as usize], // OAMDATA
            0x2007 => self.read_memory(), // PPUDATA

            _ => {
                println!("Unhandled PPU read at: ${:04X}", addr);
                0
            }
        };

        // TODO: simulate open bus
        // self.last_byte_read = result;

        result
    }

    pub fn write_register(&mut self, addr: u16, value: u8) {
        match addr {
            0x2000 => self.write_to_ctrl(value),
            0x2001 => self.mask_register.update(value),
            0x2003 => self.oam_addr = value,
            0x2004 => self.write_to_oam_data(value),
            0x2005 => self.scroll_register.write_scroll(value),
            0x2006 => self.scroll_register.write_to_addr(value),
            0x2007 => self.write_memory(value),
            0x2008..=0x3FFF => {
                // Mirror $2000–$2007 every 8 bytes
                self.write_register(0x2000 + (addr % 8), value);
            }
            _ => println!("Unhandled PPU write at: {addr:04X} = {value:02X}"),
        }
    }

    pub fn tick(&mut self) -> bool {
        // --- Start of visible scanline (0–239) or pre-render line (261)
        let rendering_enabled = self.mask_register.rendering_enabled();

        // --- Pre-render scanline logic
        if self.scanline == 261 {
            if self.cycles == 1 {
                // println!("scanline = {}, cycles = {}", self.scanline, self.cycles);
                // These happen unconditionally
                self.status_register.reset_vblank_status();
                self.status_register.set_sprite_zero_hit(false);
                self.status_register.set_sprite_overflow(false);
                self.scroll_register.reset_latch();
            }

            // Scroll copy only happens when rendering is enabled
            if rendering_enabled && (280..=304).contains(&self.cycles) {
                // println!(
                //     ">> copying vertical bits: v = {:04X} <- t = {:04X} at scanline {} cycle {}",
                //     self.scroll_register.v,
                //     self.scroll_register.t,
                //     self.scanline,
                //     self.cycles
                // );
                self.scroll_register.copy_vertical_bits();
            }
        } else if (0..240).contains(&self.scanline) && rendering_enabled {
            if self.cycles == 0 {
                self.render_scanline();
            }
            if self.cycles == 256 {
                // println!(">> increment_y before: v = {:04X}", self.scroll_register.v);
                self.scroll_register.increment_y();
                // println!(">> increment_y after: v = {:04X}", self.scroll_register.v);
            }
            if self.cycles == 257 {
                self.scroll_register.copy_horizontal_bits();
            }
        }

        // --- NMI Trigger
        if self.scanline == 241 && self.cycles == 1 {
            self.status_register.set_vblank_status(true);

            if self.ctrl_register.generate_vblank_nmi() {
                if let Some(bus_ptr) = self.bus {
                    unsafe {
                        (*bus_ptr).nmi();
                    }
                }
            }
        }

        // Advance cycle
        self.cycles += 1;

        // End of scanline
        if self.cycles >= 341 {
            self.cycles = 0;
            self.scanline += 1;

            // End of frame
            if self.scanline >= 262 {
                self.scanline = 0;
                return true; // Frame ready to display
            }
        }
        false // Frame not ready to display
    }

    pub fn run_until_vblank(&mut self) {
        // Tick the PPU until we enter vblank at beginning scanline 241
        while !(self.scanline == 241 && self.cycles == 1) {
            self.tick();
        }
    }

    pub fn write_to_oam_dma(&mut self, data: &[u8; 256]) {
        for x in data.iter() {
            self.oam_data[self.oam_addr as usize] = *x;
            self.oam_addr = self.oam_addr.wrapping_add(1);
        }
    }
}

// Private implementations
impl PPU {
    #[inline]
    fn render_scanline(&mut self) {
        // DEBUG testing...
        assert!(self.scanline < 240);
        if self.scanline >= 240 {
            return; // Prevent writing to invalid scanlines
        }

        if self.scanline == 0 && self.cycles % 8 == 0 {
            println!("v during scanline 0, cycle {}: {:04X}", self.cycles, self.scroll_register.v);
        }

        let scanline = self.scanline;

        for i in 0..256 {
            // Use current v address
            let v = self.scroll_register.v;
            let fine_x = self.scroll_register.x;

            // Fetch the correct bit for this pixel
            let bit_index = ((i + fine_x as usize) % 8) as u8;

            let bg_color = self.get_background_pixel(v, bit_index);
            let sprite = self.get_sprite_pixel(i, scanline);

            // Draw the pixel
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

            // Only increment coarse X every 8 pixels (tile boundary)
            if (i + 1) % 8 == 0 {
                self.scroll_register.increment_x();
            }

            // TODO: REMOVE debugging
            // if scanline == 239 {
            //     println!(
            //         "i={}, v={:04X}, bit_index={}",
            //         i, v, bit_index
            //     );
            // }
        }
    }

    #[inline]
    fn get_background_pixel(&mut self, v: u16, fine_x: u8) -> u8 {
        let coarse_x = v & 0b11111;
        let coarse_y = (v >> 5) & 0b11111;
        let fine_y = (v >> 12) & 0b111;

        let nametable_addr = 0x2000 | (v & 0x0FFF); // Coarse X/Y and NT bits
        let tile_index = self.ram[self.mirror_ram_addr(nametable_addr) as usize];

        let pattern_addr =
            self.ctrl_register.background_pattern_addr() + (tile_index as u16) * 16 + fine_y;
        let low_plane = self.chr_read(pattern_addr);
        let high_plane = self.chr_read(pattern_addr + 8);

        let shift = 7 - fine_x;
        let low_bit = (low_plane >> shift) & 1;
        let high_bit = (high_plane >> shift) & 1;
        let color_index = (high_bit << 1) | low_bit;

        if color_index == 0 {
            return self.palette_table[0]; // universal bg color
        }

        // Attribute table calculation
        let attribute_addr = 0x23C0 | (v & 0x0C00) | ((coarse_y >> 2) << 3) | (coarse_x >> 2);
        let attribute_byte = self.ram[self.mirror_ram_addr(attribute_addr) as usize];

        let quadrant = ((coarse_y % 4) / 2) * 2 + ((coarse_x % 4) / 2);
        let palette_bits = (attribute_byte >> (quadrant * 2)) & 0b11;

        let palette_addr = 0x3F00 + (palette_bits as u16) * 4 + (color_index as u16);

        // TODO: REMOVE debugging
        // if self.scanline == 239 {
        //     println!(
        //         "\ttile_index={:02X}, chr_addr={:04X}",
        //         tile_index, pattern_addr
        //     );
        // }

        self.palette_table[(palette_addr & 0x1F) as usize]
    }

    #[inline]
    /// Returns (palette_index, is_sprite_zero)
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

            let pattern_addr = match sprite_height {
                16 => {
                    // 8x16 sprite mode
                    let table = tile_index & 0x01;
                    let index = tile_index & 0xFE;
                    let base = (table as u16) << 12;
                    base + (index as u16) * 16 + (fine_y as u16)
                }
                _ => {
                    // 8x8 sprite mode
                    let base = self.ctrl_register.sprite_pattern_addr();
                    base + (tile_index as u16) * 16 + (fine_y as u16)
                }
            };

            let low = self.chr_read(pattern_addr);
            let high = self.chr_read(pattern_addr + 8);

            let bit = 7 - fine_x;
            let low_bit = (low >> bit) & 1;
            let high_bit = (high >> bit) & 1;
            let color_index = (high_bit << 1) | low_bit;

            if color_index == 0 {
                continue; // transparent pixel
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
        let addr = self.scroll_register.get_addr();

        let result = match addr {
            0..=0x1FFF => {
                let result = self.internal_data;
                self.internal_data = self.chr_read(addr);
                result
            }
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
                // NOTE: This is a PPU quirk.
                // When ADDR is in palette memory, it returns that value immediately
                // AND updates the internal buffer to a mirrored name-table value

                // Palette RAM (32 bytes mirrored every $20)
                let mirrored_addr = self.mirror_palette_addr(addr);
                let result = self.palette_table[mirrored_addr];

                // Quirk cont.: Address is mirrored down into nametable space
                let mirrored_vram_addr = addr & 0x2FFF;
                self.internal_data = self.ram[self.mirror_ram_addr(mirrored_vram_addr) as usize];

                result
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
        let addr = self.scroll_register.get_addr();

        match addr {
            0x0000..=0x1FFF => {
                self.chr_write(addr, value);
            }
            0x2000..=0x2FFF | 0x3000..=0x3EFF => {
                let mirrored = self.mirror_ram_addr(addr);
                self.ram[mirrored as usize] = value;
            }
            0x3F00..=0x3FFF => {
                let mut palette_addr = self.mirror_palette_addr(addr);

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

    fn set_oam_addr(&mut self, value: u8) {
        self.oam_addr = value;
    }

    fn write_to_oam_data(&mut self, value: u8) {
        self.oam_data[self.oam_addr as usize] = value;
        self.oam_addr = self.oam_addr.wrapping_add(1);
    }

    fn write_to_ctrl(&mut self, value: u8) {
        self.ctrl_register.update(value);

        // Bits 0-1 control the base nametable, which go into bits 10 and 11 of t
        self.scroll_register.t =
            (self.scroll_register.t & 0b1110011111111111) | (((value as u16) & 0b11) << 10);
    }

    fn increment_addr(&mut self) {
        self.scroll_register
            .increment_addr(self.ctrl_register.addr_increment());
    }

    pub fn mirror_palette_addr(&mut self, addr: u16) -> usize {
        let mut index = (addr - 0x3F00) % 0x20;
        if index >= 0x10 && index % 4 == 0 {
            index -= 0x10;
        }
        index as usize
    }

    pub fn mirror_ram_addr(&self, addr: u16) -> u16 {
        let mirrored_addr = addr & 0x2FFF;
        let index = mirrored_addr - 0x2000;

        let table = index / 0x400;
        let offset = index % 0x400;

        match self.mirroring() {
            Mirroring::Vertical => {
                // NT0 and NT2 share, NT1 and NT3 share
                match table {
                    0 | 2 => offset,         // NT0 or NT2
                    1 | 3 => offset + 0x400, // NT1 or NT3
                    _ => unreachable!(),
                }
            }
            Mirroring::Horizontal => {
                // NT0 and NT1 share, NT2 and NT3 share
                match table {
                    0 | 1 => offset,         // NT0 or NT1
                    2 | 3 => offset + 0x400, // NT2 or NT3
                    _ => unreachable!(),
                }
            }
            Mirroring::FourScreen => {
                index % 0x800 // TODO
            }
            Mirroring::Single0 => {
                // always map to $2000 (NT0)
                offset
            }
            Mirroring::Single1 => {
                // always map to $2400 (NT1)
                offset + 0x400
            }
            _ => unimplemented!(),
        }
    }
}

// TODO: Go through these old tests at some point...

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
