mod background;
mod mod_tests;
mod registers;
mod sprites;

use crate::cartridge::Cartridge;
use crate::ppu::registers::control_register::ControlRegister;
use crate::ppu::registers::mask_register::MaskRegister;
use crate::ppu::registers::scroll_register::ScrollRegister;
use crate::ppu::registers::status_register::StatusRegister;
use crate::rom::Mirroring;

const PRIMARY_OAM_SIZE: usize = 256;
const SECONDARY_OAM_SIZE: usize = 32;
const RAM_SIZE: usize = 2048;
const NAME_TABLE_SIZE: usize = 0x400; // Size of each nametable (1 KB)
const PALETTE_SIZE: usize = 0x20; // Size of the palette memory

pub trait PpuBusInterface {
    fn chr_read(&mut self, addr: u16) -> u8;
    fn chr_write(&mut self, addr: u16, value: u8);
    fn mirroring(&mut self) -> Mirroring;
    fn nmi(&mut self);
}

enum PaletteKind {
    Background,
    Sprite,
}

pub struct PPU {
    pub cycles: usize,
    pub scanline: usize,

    bus: Option<*mut dyn PpuBusInterface>,
    pub ram: [u8; RAM_SIZE], // $2007 (R - latched)
    internal_data: u8,
    frame_is_odd: bool,

    pub palette_table: [u8; PALETTE_SIZE],

    pub ctrl_register: ControlRegister,  // $2000 (W)
    pub mask_register: MaskRegister,     // $2001 (w)
    pub status_register: StatusRegister, // $2002 (R)
    pub scroll_register: ScrollRegister, // $2005 / $2006 - (write latched)
    pub frame_buffer: [u8; 256 * 240],

    pub oam_addr: u8,                            // $2003 (W)
    pub oam_data: [u8; PRIMARY_OAM_SIZE],        // $2004 (R/W) Object Attribute Memory
    pub secondary_oam: [u8; SECONDARY_OAM_SIZE], // holds up to 8 sprites (8 × 4 bytes)

    // Sprite evaluation & Registers
    pub sprite_pattern_low: [u8; 8],  // pattern bits plane 0
    pub sprite_pattern_high: [u8; 8], // pattern bits plane 1
    pub sprite_x_counter: [u8; 8],    // x delay counter for each sprite
    pub sprite_attributes: [u8; 8],   // palette + flipping + priority
    pub sprite_count: usize,
    sprite_zero_in_range: bool,

    // Background Registers & latches
    bg_pattern_shift_low: u16,
    bg_pattern_shift_high: u16,
    bg_attr_shift_low: u16,
    bg_attr_shift_high: u16,

    bg_attr_latch_low: u8,
    bg_attr_latch_high: u8,

    next_tile_id: u8,
    next_tile_attr: u8,
    next_tile_lsb: u8,
    next_tile_msb: u8,
}

impl PPU {
    pub fn new() -> Self {
        PPU {
            bus: None,
            ram: [0; RAM_SIZE],
            cycles: 0,
            scanline: 0,
            internal_data: 0,
            frame_is_odd: true,

            ctrl_register: ControlRegister::from_bits_truncate(0b0),
            mask_register: MaskRegister::new(),
            scroll_register: ScrollRegister::new(),
            status_register: StatusRegister::new(),

            frame_buffer: [0u8; 256 * 240],

            palette_table: [0; PALETTE_SIZE],
            oam_addr: 0,
            oam_data: [0; PRIMARY_OAM_SIZE],

            // During sprite evaluation
            secondary_oam: [0; SECONDARY_OAM_SIZE],

            // For rendering (shift registers)
            sprite_pattern_low: [0; 8],
            sprite_pattern_high: [0; 8],
            sprite_attributes: [0; 8],
            sprite_x_counter: [0; 8],
            sprite_count: 0,
            sprite_zero_in_range: false,

            bg_pattern_shift_low: 0,
            bg_pattern_shift_high: 0,

            bg_attr_shift_low: 0,
            bg_attr_shift_high: 0,
            bg_attr_latch_low: 0,
            bg_attr_latch_high: 0,

            next_tile_id: 0,
            next_tile_attr: 0,
            next_tile_lsb: 0,
            next_tile_msb: 0,
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
        // See: https://www.nesdev.org/w/images/default/4/4f/Ppu.svg
        // See: https://www.nesdev.org/wiki/PPU_rendering

        let dot = self.cycles;
        let scanline = self.scanline;
        let rendering_enabled = self.mask_register.rendering_enabled();
        let visible_scanline = scanline < 240;
        let prerender_scanline = scanline == 261;

        // --- Odd frame cycle skip (only if rendering enabled)
        if prerender_scanline && dot == 339 && self.frame_is_odd && rendering_enabled {
            // Skip cycle 340, wrap to next frame
            self.cycles = 0;
            self.scanline = 0;
            self.frame_is_odd = !self.frame_is_odd;
            return true; // frame complete
        }

        // --- Rendering pipeline
        if rendering_enabled {
            if (visible_scanline || prerender_scanline) && (1..=336).contains(&dot) {
                // === Shift the shift registers
                if (1..=256).contains(&dot) || (321..=336).contains(&dot) {
                    self.shift_background_registers();
                    if (1..=256).contains(&dot) {
                        // Only shift during visible pixels
                        self.shift_sprite_registers();
                    }
                }

                // === Render pixel
                if visible_scanline && (1..=256).contains(&dot) {
                    let color = self.render_dot();
                    self.frame_buffer[scanline * 256 + (dot - 1)] = color;
                }

                // === Background fetches
                match dot % 8 {
                    1 => self.fetch_name_table_byte(),
                    3 => self.fetch_attribute_byte(),
                    5 => self.fetch_tile_low_byte(),
                    7 => self.fetch_tile_high_byte(),
                    0 => {
                        self.load_background_registers();

                        // increment X-coord for next tile
                        if (8..=256).contains(&dot) || (328..=336).contains(&dot) {
                            self.scroll_register.increment_x();
                        }
                    }
                    _ => {}
                }

                // === Secondary-OAM clear (cycles 1–64)
                if visible_scanline && (1..=64).contains(&dot) {
                    let ind = ((dot - 1) / 2) as usize;
                    if dot % 2 == 0 {
                        self.secondary_oam[ind] = 0xFF;
                    }
                }
                if visible_scanline && dot == 64 {
                    self.reset_sprite_evaluation();
                }

                // === Sprite evaluation (65–256)
                if visible_scanline && (65..=256).contains(&dot) && dot % 2 == 1 {
                    self.sprite_evaluation(scanline, dot);
                }

                // === Sprite pattern fetches (257–320)
                if (257..=320).contains(&dot) && (visible_scanline || prerender_scanline) {
                    if (dot - 257) % 8 == 0 {
                        let sprite_num = (dot - 257) / 8;
                        self.sprite_fill_register(sprite_num as usize, scanline);
                    }
                }

                // === Scroll updates
                if dot == 256 {
                    self.scroll_register.increment_y();
                }
                if dot == 257 {
                    self.scroll_register.copy_horizontal_bits();

                    // Clear shift registers to avoid stale data
                    self.bg_pattern_shift_low = 0;
                    self.bg_pattern_shift_high = 0;
                    self.bg_attr_shift_low = 0;
                    self.bg_attr_shift_high = 0;
                }
                if prerender_scanline && (280..=304).contains(&dot) {
                    self.scroll_register.copy_vertical_bits();
                }
            }
        }

        // --- VBlank
        if prerender_scanline && dot == 1 {
            self.status_register.reset_vblank_status();
            self.status_register.set_sprite_zero_hit(false);
            self.status_register.set_sprite_overflow(false);
            self.scroll_register.reset_latch();
        }
        if scanline == 241 && dot == 1 {
            self.status_register.set_vblank_status(true);
            if self.ctrl_register.generate_vblank_nmi() {
                if let Some(bus_ptr) = self.bus {
                    unsafe {
                        (*bus_ptr).nmi();
                    }
                }
            }
        }

        // --- Advance cycle/scanline/frame
        let mut frame_complete = false;
        self.cycles += 1;
        if self.cycles > 340 {
            self.cycles = 0;
            self.scanline += 1;
            if self.scanline >= 262 {
                self.scanline = 0;
                frame_complete = true;
                self.frame_is_odd = !self.frame_is_odd;
            }
        }

        frame_complete
    }

    pub fn run_until_vblank(&mut self) {
        // Tick the PPU until VBLANK
        while !self
            .status_register
            .contains(StatusRegister::VBLANK_STARTED)
        {
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
    fn render_dot(&mut self) -> u8 {
        let bg_color = self.get_background_pixel();
        let (sprite_color, sprite_in_front, sprite_zero_rendered) = self.get_sprite_pixel();

        if sprite_zero_rendered && bg_color != 0 && sprite_color != 0 {
            self.status_register.set_sprite_zero_hit(true);
        }

        let final_color = if sprite_color == 0 {
            bg_color
        } else if bg_color == 0 {
            sprite_color
        } else {
            if sprite_in_front {
                sprite_color
            } else {
                bg_color
            }
        };
        final_color
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
                self.palette_table[palette_addr] = value;
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
    fn read_palette_color(&mut self, palette: u8, pixel: u8, palette_kind: PaletteKind) -> u8 {
        if pixel == 0 {
            return self.read_bus(0x3F00); // universal background color
        }

        let base = match palette_kind {
            PaletteKind::Background => 0x3F00,
            PaletteKind::Sprite => 0x3F10,
        };
        let index = base + ((palette as u16) << 2) + (pixel as u16);
        self.read_bus(index)
    }

    /// `read_bus` directs memory reads to correct sources (without any buffering)
    fn read_bus(&mut self, addr: u16) -> u8 {
        match addr {
            // Pattern table (CHR ROM/RAM) $0000-$1FFF
            0x0000..=0x1FFF => self.chr_read(addr),

            // Nametable RAM + mirrors $2000-$2FFF
            0x2000..=0x2FFF => {
                let mirrored_addr = self.mirror_ram_addr(addr);
                self.ram[mirrored_addr as usize]
            }

            // Mirrors of $2000-$2FFF: $3000-$3EFF
            0x3000..=0x3EFF => {
                let mirrored_addr = self.mirror_ram_addr(addr - 0x1000);
                self.ram[mirrored_addr as usize]
            }

            // Palette RAM indexes: $3F00-$3FFF
            0x3F00..=0x3FFF => {
                let mirrored_addr = self.mirror_palette_addr(addr);
                self.palette_table[mirrored_addr]
            }

            _ => panic!("open bus"), // TODO: Technically it's open bus or invalid
        }
    }

    fn read_status(&mut self) -> u8 {
        let data = self.status_register.value();
        self.status_register.reset_vblank_status();
        self.scroll_register.reset_latch();
        data
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
