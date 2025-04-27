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
    pub scanline: usize,
    pub frame_buffer: [u8; 256 * 240],

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

        let dot = self.cycles;
        let scanline = self.scanline;
        let rendering_enabled = self.mask_register.rendering_enabled();
        let visible_scanline = scanline < 240;
        let prerender_scanline = scanline == 261;

        // 0. "Skipped on BG+Odd"
        if scanline == 0 && dot == 0 {
            // TODO: I'm not sure what this means in the PPU timing chart
        }

        // 1. Clear VBlank flag at dot 1 of prerender
        if prerender_scanline && dot == 1 {
            self.status_register.reset_vblank_status();
            self.status_register.set_sprite_zero_hit(false);
            self.status_register.set_sprite_overflow(false);
            self.scroll_register.reset_latch();
        }

        // 2. Set VBlank at scanline 241, dot 1
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

        // 3. Background rendering
        if rendering_enabled {
            if (visible_scanline || prerender_scanline) && (1..=336).contains(&dot) {
                // === Shift background registers every cycle
                self.shift_background_registers();

                // === Fetch new background data
                match dot % 8 {
                    1 => self.fetch_name_table_byte(),
                    3 => self.fetch_attribute_byte(),
                    5 => self.fetch_tile_low_byte(),
                    7 => self.fetch_tile_high_byte(),
                    0 => self.load_background_registers(), // Load the fetched tile into shifters
                    _ => {}
                }

                // === Rendering pixel during visible area
                if visible_scanline && (1..=256).contains(&dot) {
                    let color = self.render_dot();
                    self.frame_buffer[scanline * 256 + (dot - 1)] = color;
                }

                // === Scrolling
                if (1..=256).contains(&dot) || (321..=336).contains(&dot) {
                    if dot % 8 == 0 {
                        self.scroll_register.increment_x();
                    }
                }

                if dot == 256 {
                    self.scroll_register.increment_y();
                }

                if dot == 257 {
                    self.scroll_register.copy_horizontal_bits();
                }

                if prerender_scanline && (280..=304).contains(&dot) {
                    self.scroll_register.copy_vertical_bits();
                }
            }
        }

        // 4. Advance cycle/scanline/frame
        let mut frame_complete = false;
        self.cycles += 1;
        if self.cycles > 340 {
            self.cycles = 0;
            self.scanline += 1;
            if self.scanline >= 262 {
                self.scanline = 0;
                frame_complete = true;
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
    /// `render_dot` returns color-index of bg pixel at (self.cycles, self.scanline)
    fn render_dot(&mut self) -> u8 {
        // assert_eq!(self.mask_register.rendering_enabled(), true); // TODO: remove
        if self.scanline == 0 && self.cycles == 1 {
            let coarse_x = self.scroll_register.v & 0b00000_00000_11111;
            println!("Start of frame: coarse_x = {}", coarse_x);
        }

        // Compute bit index from fine X scroll
        let fine_x = self.scroll_register.x;
        let bit = 15 - fine_x;
        let pixel_low = (self.bg_pattern_shift_low >> bit) & 1;
        let pixel_high = (self.bg_pattern_shift_high >> bit) & 1;
        let pixel = ((pixel_high << 1) | pixel_low) as u8;

        let attr_low = (self.bg_attr_shift_low >> bit) & 1;
        let attr_high = (self.bg_attr_shift_high >> bit) & 1;
        let palette_index = ((attr_high << 1) | attr_low) as u8;

        let color = self.read_palette_color(palette_index, pixel);
        color
    }

    #[deprecated]
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
    fn shift_background_registers(&mut self) {
        self.bg_pattern_shift_low <<= 1;
        self.bg_pattern_shift_high <<= 1;

        self.bg_attr_shift_low = (self.bg_attr_shift_low << 1) | self.bg_attr_latch_low as u16;
        self.bg_attr_shift_high = (self.bg_attr_shift_high << 1) | self.bg_attr_latch_high as u16;
    }

    fn load_background_registers(&mut self) {
        self.bg_pattern_shift_low =
            (self.bg_pattern_shift_low & 0xFF00) | self.next_tile_lsb as u16;
        self.bg_pattern_shift_high =
            (self.bg_pattern_shift_high & 0xFF00) | self.next_tile_msb as u16;

        // Update attribute shift registers with the latch values
        self.bg_attr_shift_low = (self.bg_attr_shift_low & 0xFF00)
            | (if self.bg_attr_latch_low != 0 {
                0xFF
            } else {
                0x00
            });
        self.bg_attr_shift_high = (self.bg_attr_shift_high & 0xFF00)
            | (if self.bg_attr_latch_high != 0 {
                0xFF
            } else {
                0x00
            });

        // Latch new values from fetched attribute byte
        self.bg_attr_latch_low = (self.next_tile_attr & 0b01) >> 0;
        self.bg_attr_latch_high = (self.next_tile_attr & 0b10) >> 1;
    }

    // called during dot % 8 == 1
    fn fetch_name_table_byte(&mut self) {
        assert_eq!(self.cycles % 8, 1); // TODO: remove this
        let addr = 0x2000 | (self.scroll_register.v & 0x0FFF);
        self.next_tile_id = self.read_bus(addr);
    }

    // called during dot % 8 == 3
    fn fetch_attribute_byte(&mut self) {
        assert_eq!(self.cycles % 8, 3); // TODO: remove this
        let v = self.scroll_register.v;

        let addr = 0x23C0
            | (v & 0x0C00)            // nametable select (bits 10–11 of v)
            | ((v >> 4) & 0b111_000)  // (coarse_y / 4) << 3
            | ((v >> 2) & 0b000_111); // (coarse_x / 4)
        let attr_byte = self.read_bus(addr);

        // Extract coarse X/Y positions from v
        let coarse_x = (v >> 0) & 0b11111;
        let coarse_y = (v >> 5) & 0b11111;

        // Determine which quadrant within the attribute byte
        let shift = ((coarse_y & 0x02) << 1) | (coarse_x & 0x02);
        self.next_tile_attr = (attr_byte >> shift) & 0b11;
    }

    // called during dot % 8 == 5
    fn fetch_tile_low_byte(&mut self) {
        assert_eq!(self.cycles % 8, 5); // TODO: remove this
        let fine_y = (self.scroll_register.v >> 12) & 0b111;
        let base = self.ctrl_register.background_pattern_addr();
        let tile_addr = base + (self.next_tile_id as u16) * 16 + fine_y;
        self.next_tile_lsb = self.read_bus(tile_addr);
    }

    // called during dot % 8 == 7
    fn fetch_tile_high_byte(&mut self) {
        assert_eq!(self.cycles % 8, 7); // TODO: remove this
        let fine_y = (self.scroll_register.v >> 12) & 0b111;
        let base = self.ctrl_register.background_pattern_addr();
        let tile_addr = base + (self.next_tile_id as u16) * 16 + fine_y + 8;
        self.next_tile_msb = self.read_bus(tile_addr);
    }

    fn read_palette_color(&mut self, palette: u8, pixel: u8) -> u8 {
        if pixel == 0 {
            return self.read_bus(0x3F00); // universal background color
        }

        let index = 0x3F00 + ((palette as u16) << 2) + (pixel as u16);
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

    /// ///////////////////////

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
