mod address_register;
mod control_register;
mod mask_register;
mod scroll_register;
mod status_register;

use crate::ppu::address_register::AddrRegister;
use crate::ppu::control_register::ControlRegister;
use crate::ppu::mask_register::MaskRegister;
use crate::ppu::scroll_register::ScrollRegister;
use crate::ppu::status_register::StatusRegister;
use crate::rom::Mirroring;

const OAM_SIZE: usize = 256;
const RAM_SIZE: usize = 2048;

pub struct PPU {
    pub chr_rom: Vec<u8>,
    pub palette_table: [u8; 32],
    pub ram: [u8; 2048],

    pub oam_data: [u8; 256], // Object Attribute Memory

    // Registers
    addr_register: AddrRegister,
    ctrl_register: ControlRegister,
    mask_register: MaskRegister,
    scroll_register: ScrollRegister,
    status_register: StatusRegister,

    internal_data: u8,
    cycles: usize,
    scanline: usize,
    nmi_interrupt: bool,

    pub mirroring: Mirroring,
}

impl PPU {
    pub fn new(chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        PPU {
            chr_rom,
            mirroring,
            ram: [0; RAM_SIZE],
            oam_data: [0; OAM_SIZE],
            palette_table: [0; 32],
            addr_register: AddrRegister::new(),
            ctrl_register: ControlRegister::from_bits_truncate(0b0),
            mask_register: MaskRegister::new(),
            scroll_register: ScrollRegister::new(),
            status_register: StatusRegister::new(),
            internal_data: 0,
            cycles: 0,
            scanline: 0,
            nmi_interrupt: false,
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
                // Trigger NMI if CPU hasn't requested a break from them
                if self.ctrl_register.generate_vblank_nmi() {
                    self.status_register.set_vblank_status(true);
                    self.nmi_interrupt = true;
                }
            }
            if self.scanline >= 262 {
                // Exit VBLANK past scanline 262
                self.scanline = 0;
                self.status_register.reset_vblank_status();
                self.nmi_interrupt = false;
            }
        }
    }

    pub fn get_nmi_status(&self) -> bool {
        self.nmi_interrupt
    }

    pub fn write_to_ppu_addr(&mut self, value: u8) {
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
            self.nmi_interrupt = true;
        }
    }
    fn increment_ram_addr(&mut self) {
        self.addr_register
            .increment(self.ctrl_register.increment_ram_addr());
    }

    pub fn read_data(&mut self) -> u8 {
        let addr = self.addr_register.get();
        self.increment_ram_addr();

        match addr {
            0..=0x1fff => {
                let result = self.internal_data;
                self.internal_data = self.chr_rom[addr as usize];
                result
            }
            0x2000..=0x2fff => {
                let result = self.internal_data;
                self.internal_data = self.ram[addr as usize];
                result
            }
            0x3000..=0x3eff => panic!("Invalid address ${:04X}. (0x3000..0x3EFF is invalid)", addr),
            0x3f00..=0x3fff => self.palette_table[(addr - 0x3f00) as usize],
            _ => panic!("Invalid access to mirrored space ${:04X}", addr),
        }
    }

    pub fn write_to_data(&mut self, value: u8) {
        let addr = self.addr_register.get();
        match addr {
            0..=0x1FFF => println!("Invalid PPU write to chr rom space ${:04X}", addr),
            0x2000..=0x2FFF => {
                self.ram[self.mirror_ram_addr(addr) as usize] = value;
            }
            0x3000..=0x3EFF => panic!("Invalid PPU write to address ${:04X}", addr),

            //Addresses $3F10/$3F14/$3F18/$3F1C are mirrors of $3F00/$3F04/$3F08/$3F0C
            0x3F10 | 0x3F14 | 0x3F18 | 0x3F1C => {
                let add_mirror = addr - 0x10;
                self.palette_table[(add_mirror - 0x3F00) as usize] = value;
            }
            0x3F00..=0x3FFF => {
                self.palette_table[(addr - 0x3F00) as usize] = value;
            }
            _ => panic!("unexpected access to mirrored space {}", addr),
        }
        self.increment_ram_addr();
    }

    pub fn mirror_ram_addr(&self, addr: u16) -> u16 {
        // Horizontal:          Vertical:
        //   [ A ] [ a ]          [ A ] [ B ]
        //   [ B ] [ b ]          [ a ] [ b ]
        let mirrored_ram = addr & 0b0010_1111_1111_1111; // mirror down 0x3000-0x3eff to 0x2000 - 0x2eff
        let ram_index = mirrored_ram - 0x2000; // to ram index
        let name_table = ram_index / 0x400; // to the name table index
        match (&self.mirroring, name_table) {
            (Mirroring::Vertical, 2) | (Mirroring::Vertical, 3) => ram_index - 0x800,
            (Mirroring::Horizontal, 1) => ram_index - 0x400,
            (Mirroring::Horizontal, 2) => ram_index - 0x400,
            (Mirroring::Horizontal, 3) => ram_index - 0x800,
            _ => ram_index,
        }
    }
}
