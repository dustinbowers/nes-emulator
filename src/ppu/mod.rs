mod address_register;
mod control_register;
mod mask_register;
mod scroll_register;
mod status_register;

use crate::ppu::address_register::AddressRegister;
use crate::ppu::control_register::ControlRegister;
use crate::ppu::mask_register::MaskRegister;
use crate::ppu::scroll_register::ScrollRegister;
use crate::ppu::status_register::StatusRegister;
use crate::rom::Mirroring;

const OAM_SIZE: usize = 256;
const RAM_SIZE: usize = 2048;
const NAME_TABLE_SIZE: usize = 0x400; // Size of each nametable (1 KB)
const PALETTE_SIZE: usize = 0x20; // Size of the palette memory

pub struct PPU {
    pub chr_rom: Vec<u8>,
    pub palette_table: [u8; PALETTE_SIZE],
    pub ram: [u8; RAM_SIZE],

    pub oam_addr: u8,
    pub oam_data: [u8; 256], // Object Attribute Memory

    // Registers
    pub addr_register: AddressRegister,
    pub ctrl_register: ControlRegister,
    pub mask_register: MaskRegister,
    pub scroll_register: ScrollRegister,
    pub status_register: StatusRegister,

    internal_data: u8,
    pub cycles: usize,
    scanline: usize,
    pub nmi_interrupt: Option<u8>,

    pub mirroring: Mirroring,
}

impl PPU {
    pub fn new(chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        PPU {
            chr_rom,
            mirroring,
            ram: [0; RAM_SIZE],
            oam_addr: 0,
            oam_data: [0; OAM_SIZE],
            palette_table: [0; 32],
            addr_register: AddressRegister::new(),
            ctrl_register: ControlRegister::from_bits_truncate(0b0),
            mask_register: MaskRegister::new(),
            scroll_register: ScrollRegister::new(),
            status_register: StatusRegister::new(),
            internal_data: 0,
            cycles: 0,
            scanline: 0,
            nmi_interrupt: None,
        }
    }
    pub fn tick(&mut self, cycles: usize) {
        self.cycles += cycles;
        if self.cycles >= 341 {
            // 341 cycles per scanline
            self.scanline += 1;
            self.cycles -= 341;
            if self.scanline == 241 {
                // Enter VBLANK on scanline 241
                self.status_register.set_vblank_status(true);
                // Trigger NMI if CPU hasn't requested a break from them
                if self.ctrl_register.generate_vblank_nmi() {
                    self.nmi_interrupt = Some(1);
                }
            }
            if self.scanline >= 262 {
                // Exit VBLANK past scanline 262
                self.scanline = 0;
                self.status_register.reset_vblank_status();
                self.nmi_interrupt = None;
            }
        }
    }

    pub fn read_status(&mut self) -> u8 {
        let data = self.status_register.value();
        self.status_register.reset_vblank_status();
        self.addr_register.reset_latch();
        self.scroll_register.reset_latch();
        data
    }

    pub fn get_nmi_status(&mut self) -> Option<u8> {
        self.nmi_interrupt.take()
    }

    pub fn set_oam_addr(&mut self, value: u8) {
        self.oam_addr = value;
    }

    pub fn write_to_oam_data(&mut self, value: u8) {
        self.oam_data[self.oam_addr as usize] = value;
        self.oam_addr = self.oam_addr.wrapping_add(1);
    }

    pub fn write_to_oam_dma(&mut self, data: &[u8; 256]) {
        for x in data.iter() {
            self.oam_data[self.oam_addr as usize] = *x;
            self.oam_addr = self.oam_addr.wrapping_add(1);
        }
    }

    pub fn read_oam_data(&self) -> u8 {
        self.oam_data[self.oam_addr as usize]
    }

    pub fn write_to_scroll(&mut self, value: u8) {
        self.scroll_register.write(value);
    }

    pub fn set_ppu_addr(&mut self, value: u8) {
        // println!("ppu.set_ppu_addr(${:02X})", value);
        self.addr_register.update(value);
    }
    pub fn write_to_ctrl(&mut self, value: u8) {
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
    pub fn write_to_mask(&mut self, value: u8) {
        self.mask_register.update(value);
    }

    fn increment_ram_addr(&mut self) {
        self.addr_register
            .increment(self.ctrl_register.increment_ram_addr());
    }

    pub fn read_data(&mut self) -> u8 {
        let mut addr = self.addr_register.get();
        self.increment_ram_addr();

        // Handle VRAM Mirroring (0x3000-0x3EFF → 0x2000-0x2EFF)
        if (0x3000..=0x3EFF).contains(&addr) {
            addr -= 0x1000;
        }

        match addr {
            0..=0x1FFF => {
                let result = self.internal_data;
                self.internal_data = *self.chr_rom.get(addr as usize).unwrap_or(&0);
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
            0x3F10 | 0x3F14 | 0x3F18 | 0x3F1C => {
                // let mirror_address = addr - 0x10;
                let mirror_address = addr & 0x3F0F;
                self.palette_table[(mirror_address - 0x3F00) as usize]
            }
            0x3F00..=0x3FFF => {
                let mirrored_address = addr & 0x3F1F;
                self.palette_table[(mirrored_address - 0x3F00) as usize] // No buffering for palette reads
            }
            _ => panic!("Invalid access to mirrored space ${:04X}", addr),
        }
    }

    pub fn write_to_data(&mut self, value: u8) {
        let addr = self.addr_register.get();

        match addr {
            0..=0x1FFF => {
                println!("Warning: Attempted write to CHR ROM space at {:#04X}", addr);
            }
            0x2000..=0x2FFF => {
                let mirrored_addr = self.mirror_ram_addr(addr);
                self.ram[mirrored_addr as usize] = value;
            }
            0x3000..=0x3EFF => {
                // unimplemented!("PPU write to invalid address: {:#04X}", addr);
                let mirrored_addr = self.mirror_ram_addr(addr);
                self.ram[mirrored_addr as usize] = value;
            }
            0x3F10 | 0x3F14 | 0x3F18 | 0x3F1C => {
                let mirror_addr = addr & 0x3F0F;
                self.palette_table[(mirror_addr - 0x3F00) as usize] = value;
            }
            0x3F00..=0x3FFF => {
                //fails when addr = $3FE0
                let mirrored_addr = addr & 0x3F1F;
                self.palette_table[(mirrored_addr - 0x3F00) as usize] = value;
            }
            _ => panic!("Unexpected access to mirrored space: {:#06X}", addr),
        }

        self.increment_ram_addr();
    }

    pub fn read_bytes_raw(&mut self, address: usize, size: usize) -> Vec<u8> {
        self.ram[address..=address + size].to_vec()
    }

    pub fn mirror_ram_addr(&self, addr: u16) -> u16 {
        let mirrored_ram = addr & 0b10111111111111; // mirror down 0x3000-0x3eff to 0x2000 - 0x2eff
        let ram_index = mirrored_ram - 0x2000;
        let name_table = ram_index / 0x400;
        match (&self.mirroring, name_table) {
            (Mirroring::Vertical, 2) | (Mirroring::Vertical, 3) => ram_index - 0x800,
            (Mirroring::Horizontal, 2) => ram_index - 0x400,
            (Mirroring::Horizontal, 1) => ram_index - 0x400,
            (Mirroring::Horizontal, 3) => ram_index - 0x800,
            _ => ram_index,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_empty_ppu() -> PPU {
        PPU::new(vec![0; 4096], Mirroring::Vertical)
    }

    #[test]
    fn test_write_chr_rom_space() {
        let mut ppu = create_empty_ppu();
        ppu.write_to_data(0x55);
        // Expecting no change in CHR ROM since it's read-only (unless CHR RAM is supported)
        assert_ne!(ppu.chr_rom[0], 0x55);
    }

    #[test]
    fn test_write_nametable_ram() {
        let mut ppu = create_empty_ppu();
        ppu.addr_register.set(0x2005);
        ppu.write_to_data(0xAB);
        assert_eq!(ppu.ram[ppu.mirror_ram_addr(0x2005) as usize], 0xAB);
    }

    #[test]
    fn test_write_mirrored_nametable_ram() {
        let mut ppu = create_empty_ppu();
        ppu.addr_register.set(0x3005);
        ppu.write_to_data(0xCD);
        assert_eq!(ppu.ram[ppu.mirror_ram_addr(0x3005) as usize], 0xCD);
    }

    #[test]
    fn test_write_palette_table_direct() {
        let mut ppu = create_empty_ppu();
        ppu.addr_register.set(0x3F00);
        ppu.write_to_data(0x12);
        assert_eq!(ppu.palette_table[0], 0x12);
    }

    #[test]
    fn test_write_palette_table_mirrored() {
        let mut ppu = create_empty_ppu();
        ppu.addr_register.set(0x3F10);
        ppu.write_to_data(0x34);
        assert_eq!(ppu.palette_table[0], 0x34); // Should be mirrored to 0x3F00
    }
}
