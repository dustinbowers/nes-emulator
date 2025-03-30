use bitflags::bitflags;

bitflags! {

/* See: https://www.nesdev.org/wiki/PPU_registers#PPUCTRL
    7  bit  0
    ---- ----
    VPHB SINN
    |||| ||||
    |||| ||++- Base nametable address
    |||| ||    (0 = $2000; 1 = $2400; 2 = $2800; 3 = $2C00)
    |||| |+--- VRAM address increment per CPU read/write of PPUDATA
    |||| |     (0: add 1, going across; 1: add 32, going down)
    |||| +---- Sprite pattern table address for 8x8 sprites
    ||||       (0: $0000; 1: $1000; ignored in 8x16 mode)
    |||+------ Background pattern table address (0: $0000; 1: $1000)
    ||+------- Sprite size (0: 8x8 pixels; 1: 8x16 pixels – see PPU OAM#Byte 1)
    |+-------- PPU master/slave select
    |          (0: read backdrop from EXT pins; 1: output color on EXT pins)
    +--------- Vblank NMI enable (0: off, 1: on)
 */
   pub struct ControlRegister: u8 {
       const NAMETABLE1              = 0b00000001;
       const NAMETABLE2              = 0b00000010;
       const VRAM_ADD_INCREMENT      = 0b00000100;
       const SPRITE_PATTERN_ADDR     = 0b00001000;
       const BACKROUND_PATTERN_ADDR  = 0b00010000;
       const SPRITE_SIZE             = 0b00100000;
       const MASTER_SLAVE_SELECT     = 0b01000000;
       const GENERATE_NMI            = 0b10000000;
   }
}

impl ControlRegister {
    pub fn new() -> Self {
        ControlRegister::from_bits_truncate(0b00000000)
    }

    pub fn increment_ram_addr(&self) -> u8 {
        match self.contains(ControlRegister::VRAM_ADD_INCREMENT) {
            true => { 32 }
            false => { 1 }
        }
    }

    pub fn update(&mut self, data: u8) {
        *self = ControlRegister::from_bits_truncate(data);
    }
}
