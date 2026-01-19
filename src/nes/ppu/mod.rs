use crate::nes::cartridge::rom::Mirroring;
use crate::nes::ppu::registers::control_register::ControlRegister;
use crate::nes::ppu::registers::decay_register::DecayRegister;
use crate::nes::ppu::registers::mask_register::MaskRegister;
use crate::nes::ppu::registers::scroll_register::ScrollRegister;
use crate::nes::ppu::registers::status_register::StatusRegister;
use crate::nes::tracer::traceable::Traceable;
use crate::{trace, trace_obj, trace_ppu_event};

mod background;
mod mod_tests;
pub mod registers;
mod sprites;

const PRIMARY_OAM_SIZE: usize = 256;
const SECONDARY_OAM_SIZE: usize = 32;
const RAM_SIZE: usize = 2048;
const NAME_TABLE_SIZE: u16 = 0x400; // Size of each nametable (1 KB)
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
    suppress_vblank: bool,
    nmi_fired_this_vblank: bool,
    instant_nmi_pending: bool,
    pub global_ppu_ticks: usize,
    pub vblank_ticks: usize, // TODO: remove this
    prerender_rendering_enabled: bool,
    last_2002_read_dot: usize,
    last_2002_read_scanline: usize,

    bus: Option<*mut dyn PpuBusInterface>,
    pub v_ram: [u8; RAM_SIZE], // $2007 (R - latched)
    internal_data: u8,
    frame_is_odd: bool,
    last_byte_read: DecayRegister,

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
            v_ram: [0; RAM_SIZE],
            cycles: 0,
            scanline: 261,
            instant_nmi_pending: false,
            suppress_vblank: false,
            nmi_fired_this_vblank: false,
            global_ppu_ticks: 0,
            vblank_ticks: 0, // TODO: Remove this
            prerender_rendering_enabled: false,
            last_2002_read_dot: 0,
            last_2002_read_scanline: 0,

            internal_data: 0,
            frame_is_odd: false,
            last_byte_read: DecayRegister::new(5_369_318),

            ctrl_register: ControlRegister::new(),
            mask_register: MaskRegister::new(),
            scroll_register: ScrollRegister::new(),
            status_register: StatusRegister::new(),

            frame_buffer: [0u8; 256 * 240],

            // Blarrg's startup palette
            palette_table: [
                0x09, 0x01, 0x00, 0x01, 0x00, 0x02, 0x02, 0x0D, 0x08, 0x10, 0x08, 0x24, 0x00, 0x00,
                0x04, 0x2C, 0x09, 0x01, 0x34, 0x03, 0x00, 0x04, 0x00, 0x14, 0x08, 0x3A, 0x00, 0x02,
                0x00, 0x20, 0x2C, 0x08,
            ],
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

    pub fn reset(&mut self) {
        self.v_ram = [0; RAM_SIZE];
        self.cycles = 0;
        self.scanline = 261;
        self.instant_nmi_pending = false;
        self.suppress_vblank = false;
        self.nmi_fired_this_vblank = false;
        self.global_ppu_ticks = 0;
        self.vblank_ticks = 0;
        self.prerender_rendering_enabled = false;
        self.last_2002_read_dot = 0;
        self.last_2002_read_scanline = 0;

        self.internal_data = 0;
        self.frame_is_odd = false;
        self.last_byte_read = DecayRegister::new(5_369_318);

        self.ctrl_register = ControlRegister::new();
        self.mask_register = MaskRegister::new();
        self.status_register = StatusRegister::new();
        self.scroll_register = ScrollRegister::new();

        self.frame_buffer = [0u8; 256 * 240];

        // Blarrg's startup palette
        self.palette_table = [
            0x09, 0x01, 0x00, 0x01, 0x00, 0x02, 0x02, 0x0D, 0x08, 0x10, 0x08, 0x24, 0x00, 0x00,
            0x04, 0x2C, 0x09, 0x01, 0x34, 0x03, 0x00, 0x04, 0x00, 0x14, 0x08, 0x3A, 0x00, 0x02,
            0x00, 0x20, 0x2C, 0x08,
        ];

        self.oam_addr = 0;
        self.oam_data = [0; PRIMARY_OAM_SIZE];

        // During sprite evaluation
        self.secondary_oam = [0; SECONDARY_OAM_SIZE];

        // For rendering (shift registers)
        self.sprite_pattern_low = [0; 8];
        self.sprite_pattern_high = [0; 8];
        self.sprite_attributes = [0; 8];
        self.sprite_x_counter = [0; 8];
        self.sprite_count = 0;
        self.sprite_zero_in_range = false;

        self.bg_pattern_shift_low = 0;
        self.bg_pattern_shift_high = 0;

        self.bg_attr_shift_low = 0;
        self.bg_attr_shift_high = 0;
        self.bg_attr_latch_low = 0;
        self.bg_attr_latch_high = 0;

        self.next_tile_id = 0;
        self.next_tile_attr = 0;
        self.next_tile_lsb = 0;
        self.next_tile_msb = 0;
    }

    /// `connect_bus` MUST be called after constructing PPU
    pub fn connect_bus(&mut self, bus: *mut dyn PpuBusInterface) {
        self.bus = Some(bus);
    }

    pub fn read_register(&mut self, addr: u16) -> u8 {
        assert!(addr >= 0x2000);
        assert!(addr <= 0x3FFF);
        let reg = 0x2000 + (addr & 7); // mirror every 8 bytes

        match reg {
            0x2000 | 0x2001 | 0x2003 | 0x2005 | 0x2006 => {
                // write-only registers return open bus
                self.last_byte_read.output()
            }
            0x2002 => {
                // PPUSTATUS - latch current status before side effects
                let status = self.status_register.bits();
                let had_vblank = (status & 0x80) != 0;
                self.last_2002_read_scanline = self.scanline;
                self.last_2002_read_dot = self.cycles + 1;

                // From return value:
                // bits 7–5 = real status
                // bits 4–0 = open bus (last read)
                let result = (status & 0xE0) | (self.last_byte_read.output() & 0x1F);
                // trace!("[READ $2002] last_2002_read_cycle = {}", self.global_ppu_ticks);

                // side effects happen AFTER capturing status
                if had_vblank {
                    self.status_register.reset_vblank_status();
                    self.scroll_register.reset_latch();
                }

                // update open bus with what was read
                self.last_byte_read.set(reg, result);

                // Quirk: reading $2002 one PPU clock before VBL suppresses VBL for that frame.
                // if self.scanline == 241 && (self.cycles == 0 || self.cycles == 1) && !had_vblank {
                if self.scanline == 241 && (self.cycles == 0) && !had_vblank {
                    self.suppress_vblank = true;
                }
                result
            }
            0x2004 => {
                let value = self.oam_data[self.oam_addr as usize];
                self.last_byte_read.set(reg, value);
                value
            }
            0x2007 => {
                let value = self.read_memory(true);
                self.last_byte_read.set(reg, value);
                value
            }
            _ => {
                // fallback open bus
                self.last_byte_read.output()
            }
        }
    }

    pub fn write_register(&mut self, addr: u16, value: u8) {
        assert!(addr >= 0x2000);
        assert!(addr <= 0x3FFF);
        let reg = 0x2000 + (addr & 7); // mirror

        match reg {
            0x2000 => self.write_to_ctrl(value),
            0x2001 => self.mask_register.update(value),
            0x2003 => self.oam_addr = value,
            0x2004 => self.write_to_oam_data(value),
            0x2005 => self.scroll_register.write_scroll(value),
            0x2006 => self.scroll_register.write_to_addr(value),
            0x2007 => self.write_memory(value),
            _ => {
                println!("Unhandled PPU write at: {addr:04X} = {value:02X}");
            }
        }
        self.last_byte_read.set(addr, value);
    }

    /// Advance the PPU by 1 dot
    pub fn tick(&mut self) -> bool {
        let dot = self.cycles + 1;
        let scanline = self.scanline;
        let prerender_scanline = scanline == 261;
        let visible_scanline = scanline < 240;

        // VBLANK clear at start of prerender scanline (dot 1)
        // if prerender_scanline && dot == 1 {
        if prerender_scanline && self.cycles == 1 {
            trace_ppu_event!(
                "VBLANK CLEAR  frame={} scanline={} dot={} ppu_cycle={}",
                self.frame_is_odd as u8,
                scanline,
                dot,
                self.global_ppu_ticks
            );

            self.prerender_rendering_enabled = self.mask_register.rendering_enabled();
            self.nmi_fired_this_vblank = false;
            self.status_register.reset_vblank_status();
            self.status_register.set_sprite_zero_hit(false);
            self.status_register.set_sprite_overflow(false);
            self.scroll_register.reset_latch();
            self.reset_sprite_evaluation();
        }

        if self.status_register.vblank_active() {
            self.vblank_ticks += 1;
        }

        // Rendering pipeline
        if self.mask_register.rendering_enabled() && (visible_scanline || prerender_scanline) {
            // Background fetches (dots 1..=256, 321..=336)
            if (1..=256).contains(&dot) || (321..=336).contains(&dot) {
                self.shift_background_registers();
                if visible_scanline && (1..=256).contains(&dot) {
                    self.shift_sprite_registers();
                }
            }

            // Pixel rendering (visible scanlines, dots 1..=256)
            if visible_scanline && (1..=256).contains(&dot) {
                let color = self.render_dot();
                self.frame_buffer[scanline * 256 + (dot - 1)] = color;
            }

            // Background fetch sequence
            match dot % 8 {
                1 => self.fetch_name_table_byte(),
                3 => self.fetch_attribute_byte(),
                5 => self.fetch_tile_low_byte(),
                7 => self.fetch_tile_high_byte(),
                0 => {
                    self.load_background_registers();
                    if (8..=256).contains(&dot) || (328..=336).contains(&dot) {
                        self.scroll_register.increment_x();
                    }
                }
                _ => {}
            }

            // Secondary OAM clear (dots 1–64)
            if visible_scanline && (1..=64).contains(&dot) && dot.is_multiple_of(2) {
                let ind = (dot - 1) / 2;
                self.secondary_oam[ind] = 0xFF;
            }

            // Reset sprite evaluation at dot 64
            if visible_scanline && dot == 64 {
                self.reset_sprite_evaluation();
            }

            // Sprite evaluation (dots 65–256, odd dots)
            if visible_scanline && (65..=256).contains(&dot) && dot % 2 == 1 {
                self.sprite_evaluation(scanline, dot);
            }

            // Sprite pattern fetches (dots 257–320, every 8 dots)
            if (257..=320).contains(&dot)
                && (dot - 257).is_multiple_of(8)
                && (visible_scanline || prerender_scanline)
            {
                let sprite_num = (dot - 257) / 8;
                self.sprite_fill_register(sprite_num, scanline);
            }

            // Scroll updates
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

        // VBLANK set at start of scanline 241 (dot 1)
        // NMI edge
        // if scanline == 241 && dot == 1 {
        if scanline == 241 && self.cycles == 1 {
            self.vblank_ticks = 0;

            let suppress_nmi_due_to_read = self.last_2002_read_scanline == 241
                && (self.last_2002_read_dot >= 1 && self.last_2002_read_dot <= 3);


            trace_ppu_event!(
                "VBLANK SET    frame={} scanline={} dot={} ppu_cycle={} suppress_nmi={}",
                self.frame_is_odd as u8,
                scanline,
                dot,
                self.global_ppu_ticks,
                suppress_nmi_due_to_read
            );

            if !self.suppress_vblank {
                self.status_register.set_vblank_started();
            }

            if self.ctrl_register.nmi_enabled()
                && !suppress_nmi_due_to_read
                && !self.nmi_fired_this_vblank
            {
                self.nmi_fired_this_vblank = true;
                trace_ppu_event!(
                    "NMI FIRED     frame={} scanline={} dot={} ppu_cycle={} nmi_fired_this_vblank={}",
                    self.frame_is_odd as u8,
                    scanline,
                    dot,
                    self.global_ppu_ticks,
                    self.nmi_fired_this_vblank
                );
                if let Some(bus_ptr) = self.bus {
                    unsafe {
                        (*bus_ptr).nmi();
                    }
                }
            }
        }

        // Notify CPU of NMI if one is triggered from a write to CTRL
        if self.instant_nmi_pending && self.ctrl_register.nmi_enabled() {
            if let Some(bus_ptr) = self.bus {
                unsafe { (*bus_ptr).nmi(); }
            }
            self.instant_nmi_pending = false;
        }

        // Odd-frame skip
        if prerender_scanline && dot == 340 && self.frame_is_odd && self.prerender_rendering_enabled
        {
            trace_ppu_event!(
                "ODD SKIP      frame={} scanline={} dot={} ppu_cycle={}",
                self.frame_is_odd as u8,
                scanline,
                dot,
                self.global_ppu_ticks
            );

            // NES skips dot 339 on odd frames with rendering enabled at prerender start
            self.global_ppu_ticks += 1;
            self.cycles = 0;
            self.scanline = 0;
            self.suppress_vblank = false;
            self.frame_is_odd = !self.frame_is_odd;
            return true; // frame complete
        }

        // Prevent overflow
        if self.global_ppu_ticks > 1_000_000 {
            self.global_ppu_ticks -= 1_000_000;
        }

        // trace_obj!(&*self);

        let mut frame_complete = false;
        self.global_ppu_ticks += 1;
        self.cycles += 1;
        if self.cycles == 341 {
            self.cycles = 0;
            self.scanline += 1;

            if self.scanline > 261 {
                self.scanline = 0;
                self.suppress_vblank = false;
                frame_complete = true;
                self.frame_is_odd = !self.frame_is_odd;
                trace!(
                    "[FRAME END] SL={} dot={} frame_is_odd becomes: {}",
                    scanline, dot, self.frame_is_odd
                );
            }
        }
        frame_complete
    }

    #[cfg(test)]
    pub fn run_until_vblank(&mut self) {
        // Tick the PPU until VBLANK
        while !self
            .status_register
            .contains(StatusRegister::VBLANK_STARTED)
        {
            self.tick();
        }
    }
}

// Private implementations
impl PPU {
    fn render_dot(&mut self) -> u8 {
        // Get raw palette indices for background and sprite
        let (bg_palette_index, bg_pixel) = self.get_background_pixel();
        let (sprite_palette_index, sprite_pixel, sprite_in_front, sprite_zero_rendered) =
            self.get_sprite_pixel();

        // sprite 0 hit only if both rendering enabled, on visible scanlines and dots 1-256
        let show_bg = self.mask_register.show_background();
        let show_spr = self.mask_register.show_sprites();
        let left_bg = self.mask_register.leftmost_8pxl_background();
        let left_spr = self.mask_register.leftmost_8pxl_sprite();
        let dot = self.cycles;
        let scanline = self.scanline;

        // Only check for sprite 0 hit on visible scanlines and dots 1-256
        if show_bg && show_spr && scanline < 240 && (1..=256).contains(&dot) {
            // Leftmost 8px masking: if in dots 1-8, require both leftmost bits enabled
            if (dot > 8 || (left_bg && left_spr))
                && (sprite_zero_rendered && bg_palette_index != 0)
                && !self
                    .status_register
                    .contains(StatusRegister::SPRITE_ZERO_HIT)
            {
                trace!(
                    "\tset_sprite_zero_hit TRUE @ scanline {} dot {}",
                    scanline, dot
                );
                self.status_register.set_sprite_zero_hit(true);
            }
        }

        // if both nonzero, sprite_in_front decides, but if sprite is behind and bg is 0, sprite shows
        let (palette, pixel, kind) = if sprite_pixel == 0 {
            // No sprite pixel - just background
            (bg_palette_index, bg_pixel, PaletteKind::Background)
        } else if bg_pixel == 0 {
            // No background pixel - sprite wins
            (sprite_palette_index, sprite_pixel, PaletteKind::Sprite)
        } else if sprite_in_front {
            // Both nonzero, sprite wins
            (sprite_palette_index, sprite_pixel, PaletteKind::Sprite)
        } else {
            // Both nonzero, sprite behind bg
            (bg_palette_index, bg_pixel, PaletteKind::Background)
        };

        self.read_palette_color(palette, pixel, kind) & 0x3F
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

    fn read_memory(&mut self, increment: bool) -> u8 {
        let addr = self.scroll_register.get_addr();

        let result = match addr {
            0..=0x1FFF => {
                let result = self.internal_data;
                self.internal_data = self.chr_read(addr);
                result
            }
            0x2000..=0x2FFF => {
                let result = self.internal_data;
                self.internal_data = self.v_ram[self.mirror_ram_addr(addr) as usize];
                result
            }
            0x3000..=0x3EFF => {
                let result = self.internal_data;
                self.internal_data = self.v_ram[self.mirror_ram_addr(addr) as usize];
                result
            }
            0x3F00..=0x3FFF => {
                // NOTE: This is a PPU quirk.
                // When ADDR is in palette memory, it returns that value immediately
                // AND updates the internal buffer to a mirrored name-table value

                // Palette RAM (32 bytes mirrored every $20)
                let palette_index = self.mirror_palette_addr(addr);
                let result = self.palette_table[palette_index];

                // Quirk cont.: Address is mirrored down into nametable space
                let mirrored_vram_addr = addr & 0x2FFF;
                self.internal_data = self.v_ram[self.mirror_ram_addr(mirrored_vram_addr) as usize];

                result
            }
            _ => {
                eprintln!("Unhandled PPU::read_memory() at {:04X}", addr);
                // TODO: Is this an open-bus? If so, return low-byte of addr
                // https://www.nesdev.org/wiki/Open_bus_behavior#PPU_open_bus
                0
            }
        };
        if increment {
            self.increment_addr();
        }
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
                self.v_ram[mirrored as usize] = value;
            }
            0x3F00..=0x3FFF => {
                let palette_addr = self.mirror_palette_addr(addr);

                // Handle mirrors of universal background color
                // match palette_addr {
                //     0x10 | 0x14 | 0x18 | 0x1C => palette_addr -= 0x10,
                //     _ => {}
                // }
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
        let mut addr = base + ((palette as u16) << 2) + (pixel as u16);
        // Mirror sprite palettes $3F10/$3F14/$3F18/$3F1C
        if addr == 0x3F10 || addr == 0x3F14 || addr == 0x3F18 || addr == 0x3F1C {
            addr -= 0x10;
        }
        self.read_bus(addr)
    }

    /// `read_bus` directs memory reads to correct sources (without any buffering)
    fn read_bus(&mut self, addr: u16) -> u8 {
        match addr {
            // Pattern table (CHR ROM/RAM) $0000-$1FFF
            0x0000..=0x1FFF => self.chr_read(addr),

            // Nametable RAM + mirrors $2000-$2FFF
            0x2000..=0x2FFF => {
                let mirrored_addr = self.mirror_ram_addr(addr);
                self.v_ram[mirrored_addr as usize]
            }

            // Mirrors of $2000-$2FFF: $3000-$3EFF
            0x3000..=0x3EFF => {
                let mirrored_addr = self.mirror_ram_addr(addr - 0x1000);
                self.v_ram[mirrored_addr as usize]
            }

            // Palette RAM indexes: $3F00-$3FFF
            0x3F00..=0x3FFF => {
                let mirrored_addr = self.mirror_palette_addr(addr);
                self.palette_table[mirrored_addr]
            }

            _ => self.last_byte_read.output(),
        }
    }

    pub fn write_to_oam_data(&mut self, value: u8) {
        self.oam_data[self.oam_addr as usize] = value;
        self.oam_addr = self.oam_addr.wrapping_add(1);
    }

    fn write_to_ctrl(&mut self, value: u8) {
        let prev_nmi_enable = self.ctrl_register.contains(ControlRegister::GENERATE_NMI);
        self.ctrl_register.update(value);

        // Immediately trigger NMI if it goes from 0->1 during VBLANK
        if !prev_nmi_enable
            && value & 0b1000_0000 != 0
            && (self.scanline >= 241 && self.cycles > 0)
            && (self.scanline <= 260 && self.cycles <= 340)
            && let Some(bus_ptr) = self.bus
        {
            self.instant_nmi_pending = true;
        }

        // Bits 0-1 control the base nametable, which go into bits 10 and 11 of t
        const NT_BITS_MASK: u16 = 0x0C00; // bits 10 and 11
        let nt = ((value as u16) & 0b11) << 10;
        self.scroll_register.t = (self.scroll_register.t & !NT_BITS_MASK) | nt;
    }

    fn increment_addr(&mut self) {
        self.scroll_register
            .increment_addr(self.ctrl_register.addr_increment());
    }

    pub fn mirror_palette_addr(&self, addr: u16) -> usize {
        // Fold full 3F00-3FFF range into 0x3F00-0x3F1F
        let mut index = (addr - 0x3F00) % 0x20;

        // Special-case mirrors: $10/$14/$18/$1C -> $00/$04/$08/$0C
        // if index & 0x03 == 0 && index >= 0x10 {
        //     index -= 0x10;
        // }
        index = match index {
            0x10 => 0x00,
            0x14 => 0x04,
            0x18 => 0x08,
            0x1C => 0x0C,
            _ => index,
        };

        index as usize
    }

    pub fn mirror_ram_addr(&self, addr: u16) -> u16 {
        let mirrored_addr = addr & 0x2FFF;
        let index = mirrored_addr - 0x2000;

        let table = index / NAME_TABLE_SIZE;
        let offset = index % NAME_TABLE_SIZE;

        match self.mirroring() {
            Mirroring::Vertical => {
                // NT0 and NT2 share, NT1 and NT3 share
                match table {
                    0 | 2 => offset,                   // NT0 or NT2
                    1 | 3 => offset + NAME_TABLE_SIZE, // NT1 or NT3
                    _ => unreachable!(),
                }
            }
            Mirroring::Horizontal => {
                // NT0 and NT1 share, NT2 and NT3 share
                match table {
                    0 | 1 => offset,                   // NT0 or NT1
                    2 | 3 => offset + NAME_TABLE_SIZE, // NT2 or NT3
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
                offset + NAME_TABLE_SIZE
            }
        }
    }
}

impl Traceable for PPU {
    fn trace_name(&self) -> &'static str {
        "PPU"
    }
    fn trace_state(&self) -> Option<String> {
        Some(format!(
            "scanline={} dot={} cpu_visible_vblank={} odd={} global_ppu_cycles={} ppu_status={:08b}",
            self.scanline,
            self.cycles,
            self.status_register
                .contains(StatusRegister::VBLANK_STARTED),
            self.frame_is_odd,
            self.global_ppu_ticks,
            self.status_register.bits()
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::nes::cartridge::mapper000_nrom::NromCart;

    fn create_empty_ppu() -> PPU {
        let cart = NromCart::new(vec![0; 0x4000], vec![0; 0x4000], Mirroring::Vertical);
        PPU::new()
    }

    #[test]
    fn test_palette_addr_mirroring() {
        let ppu = create_empty_ppu(); // adjust constructor if needed

        // All addresses 3F00–3FFF should map to 3F00–3F1F
        for addr in 0x3F00..=0x3FFF {
            let mirrored = ppu.mirror_palette_addr(addr);
            assert!(
                mirrored < 0x20,
                "Address ${:04X} mapped outside palette range: ${:02X}",
                addr,
                mirrored
            );
        }

        // Special universal background mirrors
        assert_eq!(ppu.mirror_palette_addr(0x3F10), 0x00);
        assert_eq!(ppu.mirror_palette_addr(0x3F14), 0x04);
        assert_eq!(ppu.mirror_palette_addr(0x3F18), 0x08);
        assert_eq!(ppu.mirror_palette_addr(0x3F1C), 0x0C);
    }

    #[test]
    fn test_palette_read_write_symmetry() {
        let mut ppu = create_empty_ppu();

        for addr in 0x3F00..=0x3FFF {
            let expected = (addr & 0x3F) as u8;

            // directly set vram address
            ppu.scroll_register.v = addr;
            ppu.write_memory(expected);

            // read back from same addr
            ppu.scroll_register.v = addr;
            let value = ppu.read_memory(false);

            // Palette reads mask out upper bits (open bus),
            // so compare only the lower 6 bits.
            assert_eq!(
                value & 0x3F,
                expected & 0x3F,
                "Mismatch at ${:04X}: wrote {:02X}, read {:02X}",
                addr,
                expected,
                value
            );
        }
    }

    #[test]
    fn test_ppu_palette_wrap_full_range() {
        let ppu = create_empty_ppu();

        // The full 0x3F00–0x3FFF range should mirror into 0x00–0x1F
        for addr in 0x3F00..=0x3FFF {
            let mirrored = ppu.mirror_palette_addr(addr);
            let folded_index = (addr - 0x3F00) & 0x1F;
            let expected = if folded_index >= 0x10 && (folded_index & 0x03) == 0 {
                folded_index - 0x10
            } else {
                folded_index
            };
            assert_eq!(
                mirrored as u16, expected,
                "Address ${:04X} mirrored incorrectly: got {}, expected {}",
                addr, mirrored, expected
            );
        }
    }

    #[test]
    fn test_ppu_palette_wrap_full_range2() {
        let ppu = create_empty_ppu(); // assume this creates a PPU with 32-byte palette_table

        // Check the full range $3F00–$3FFF
        for addr in 0x3F00..=0x3FFF {
            let mirrored = ppu.mirror_palette_addr(addr);

            // Base index in 0..31
            let base_index = (addr - 0x3F00) & 0x1F;

            // Apply special mirrors for universal background color
            let expected = match base_index {
                0x10 => 0x00,
                0x14 => 0x04,
                0x18 => 0x08,
                0x1C => 0x0C,
                _ => base_index,
            } as usize;

            assert_eq!(
                mirrored, expected,
                "PPU palette address mirroring failed at {:04X}: got {}, expected {}",
                addr, mirrored, expected
            );
        }
    }

    #[test]
    fn test_ppu_palette_write_read_consistency() {
        let mut ppu = create_empty_ppu();

        // Write each palette index and check reads
        for addr in 0x3F00..=0x3FFF {
            let value = (addr & 0xFF) as u8;
            ppu.scroll_register.v = addr; // set address directly
            ppu.write_memory(value);

            // Read back immediately
            ppu.scroll_register.v = addr;
            let read_value = ppu.read_memory(false);

            let mirrored = ppu.mirror_palette_addr(addr);
            assert_eq!(
                read_value, ppu.palette_table[mirrored],
                "Palette read mismatch at ${:04X}",
                addr
            );
        }
    }

    #[test]
    fn test_ppu_palette_wrap_and_specials() {
        let ppu = create_empty_ppu();

        // For each canonical index in 0..31 (i.e. offsets into $3F00-$3F1F)
        for raw_index in 0..32usize {
            // expected index after applying special-case mapping
            let expected = if raw_index >= 0x10 && (raw_index & 0x03) == 0 {
                raw_index - 0x10
            } else {
                raw_index
            };

            // Check the canonical address and all its 8 mirrors:
            // addresses are 0x3F00 + raw_index + k*0x20 for k=0..7
            for k in 0..8usize {
                let addr = 0x3F00u16.wrapping_add((raw_index as u16) + (k as u16 * 0x20));
                let idx = ppu.mirror_palette_addr(addr);
                assert_eq!(
                    idx, expected,
                    "addr {:#06X} mapped to {} but expected {}",
                    addr, idx, expected
                );
            }
        }
    }

    #[test]
    fn test_palette_address_full_3f00_3fff_mirroring() {
        let ppu = create_empty_ppu();

        // Test the entire 0x100 (256) byte range mirroring to 0x3F00-0x3F1F
        for i in 0x00..=0xFF {
            let addr = 0x3F00 + i;
            let expected_base_mirror = (i % 0x20) as usize;

            let mut final_expected = expected_base_mirror;

            // Apply sprite palette mirroring to background palette region
            if final_expected >= 0x10 {
                final_expected -= 0x10;
            }
            // Apply universal background color mirroring
            if final_expected % 4 == 0 && final_expected != 0 {
                final_expected = 0x00;
            }

            assert_eq!(
                ppu.mirror_palette_addr(addr),
                final_expected,
                "Failed at address {:#06X}",
                addr
            );
            // println!("addr = {:04X} -> {:04X}", addr, ppu.mirror_palette_addr(addr));
        }
    }

    // #[test]
    // fn test_universal_background_color_mirroring() {
    //     let ppu = create_empty_ppu();
    //
    //     // Background color entries should all point to 0x00 (3F00)
    //     assert_eq!(ppu.mirror_palette_addr(0x3F00), 0x00, "3F00 should map to 0x00");
    //     assert_eq!(ppu.mirror_palette_addr(0x3F04), 0x00, "3F04 should map to 0x00");
    //     assert_eq!(ppu.mirror_palette_addr(0x3F08), 0x00, "3F08 should map to 0x00");
    //     assert_eq!(ppu.mirror_palette_addr(0x3F0C), 0x00, "3F0C should map to 0x00");
    //
    //     // These are sprite palette background entries, which mirror to the main background palette first
    //     // and then get the background color rule applied.
    //     assert_eq!(ppu.mirror_palette_addr(0x3F10), 0x00, "3F10 should map to 0x00");
    //     assert_eq!(ppu.mirror_palette_addr(0x3F14), 0x00, "3F14 should map to 0x00");
    //     assert_eq!(ppu.mirror_palette_addr(0x3F18), 0x00, "3F18 should map to 0x00");
    //     assert_eq!(ppu.mirror_palette_addr(0x3F1C), 0x00, "3F1C should map to 0x00");
    //
    //     // Test addresses beyond the 3F00-3F1F range that also hit these points
    //     assert_eq!(ppu.mirror_palette_addr(0x3F20), 0x00, "3F20 (mirrors 3F00) should map to 0x00");
    //     assert_eq!(ppu.mirror_palette_addr(0x3F24), 0x00, "3F24 (mirrors 3F04) should map to 0x00");
    //     assert_eq!(ppu.mirror_palette_addr(0x3F30), 0x00, "3F30 (mirrors 3F10) should map to 0x00");
    //     assert_eq!(ppu.mirror_palette_addr(0x3FF0), 0x00, "3FF0 (mirrors 3F10) should map to 0x00");
    //     assert_eq!(ppu.mirror_palette_addr(0x3FF4), 0x00, "3FF4 (mirrors 3F14) should map to 0x00");
    // }

    #[test]
    fn test_non_background_color_entries() {
        let ppu = create_empty_ppu();

        // These should map to their direct mirrored address within 0x00-0x0F
        assert_eq!(ppu.mirror_palette_addr(0x3F01), 0x01);
        assert_eq!(ppu.mirror_palette_addr(0x3F02), 0x02);
        assert_eq!(ppu.mirror_palette_addr(0x3F03), 0x03);
        assert_eq!(ppu.mirror_palette_addr(0x3F05), 0x05);
        assert_eq!(ppu.mirror_palette_addr(0x3F06), 0x06);
        assert_eq!(ppu.mirror_palette_addr(0x3F07), 0x07);
        assert_eq!(ppu.mirror_palette_addr(0x3F09), 0x09);
        assert_eq!(ppu.mirror_palette_addr(0x3F0A), 0x0A);
        assert_eq!(ppu.mirror_palette_addr(0x3F0B), 0x0B);
        assert_eq!(ppu.mirror_palette_addr(0x3F0D), 0x0D);
        assert_eq!(ppu.mirror_palette_addr(0x3F0E), 0x0E);
        assert_eq!(ppu.mirror_palette_addr(0x3F0F), 0x0F);

        // Test sprite palette entries that are not background colors,
        // they should map to their equivalent in the background palette
        assert_eq!(ppu.mirror_palette_addr(0x3F11), 0x01);
        assert_eq!(ppu.mirror_palette_addr(0x3F15), 0x05);
        assert_eq!(ppu.mirror_palette_addr(0x3F1F), 0x0F);

        // Test addresses beyond the 3F00-3F1F range
        assert_eq!(
            ppu.mirror_palette_addr(0x3F21),
            0x01,
            "3F21 (mirrors 3F01) should map to 0x01"
        );
        assert_eq!(
            ppu.mirror_palette_addr(0x3F33),
            0x03,
            "3F33 (mirrors 3F13 which maps to 3F03) should map to 0x03"
        );
        assert_eq!(
            ppu.mirror_palette_addr(0x3FF7),
            0x07,
            "3FF7 (mirrors 3F17 which maps to 3F07) should map to 0x07"
        );
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
