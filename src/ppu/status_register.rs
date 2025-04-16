use bitflags::bitflags;

bitflags! {
    /* See: https://www.nesdev.org/wiki/PPU_registers#PPUSTATUS
        7  bit  0
        ---- ----
        VSOx xxxx
        |||| ||||
        |||+-++++- (PPU open bus or 2C05 PPU identifier)
        ||+------- Sprite overflow flag
        |+-------- Sprite 0 hit flag
        +--------- Vblank flag, cleared on read.
     */
    pub struct StatusRegister: u8 {
        // const UNUSED1          = 0b00000001;
        // const UNUSED2          = 0b00000010;
        // const UNUSED3          = 0b00000100;
        // const UNUSED4          = 0b00001000;
        // const UNUSED5          = 0b00010000;
        const SPRITE_OVERFLOW  = 0b00100000;
        const SPRITE_ZERO_HIT  = 0b01000000;
        const VBLANK_STARTED   = 0b10000000;
    }
}

impl StatusRegister {
    pub fn new() -> Self {
        StatusRegister::from_bits_truncate(0)
    }

    pub fn set_vblank_status(&mut self, status: bool) {
        self.set(StatusRegister::VBLANK_STARTED, status);
    }

    pub fn reset_vblank_status(&mut self) {
        self.remove(StatusRegister::VBLANK_STARTED);
    }

    pub fn set_sprite_zero_hit(&mut self, status: bool) {
        self.set(StatusRegister::SPRITE_ZERO_HIT, status);
    }

    pub fn set_sprite_overflow(&mut self, status: bool) {
        self.set(StatusRegister::SPRITE_OVERFLOW, status);
    }

    pub fn value(&mut self) -> u8 {
        self.bits()
    }
}
