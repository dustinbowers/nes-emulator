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
       const BACKGROUND_PATTERN_ADDR = 0b00010000;
       const SPRITE_SIZE             = 0b00100000;
       const MASTER_SLAVE_SELECT     = 0b01000000;
       const GENERATE_NMI            = 0b10000000;
   }
}

impl ControlRegister {
    pub fn new() -> Self {
        ControlRegister::from_bits_truncate(0b00000000)
    }

    pub fn addr_increment(&self) -> u8 {
        match self.contains(ControlRegister::VRAM_ADD_INCREMENT) {
            true => 32,
            false => 1,
        }
    }

    pub fn generate_vblank_nmi(&self) -> bool {
        self.contains(Self::GENERATE_NMI)
    }

    // pub fn get_nametable_addr(&self) -> u16 {
    //     let nt_bits = self.bits() & 0b11;
    //     match nt_bits {
    //         0b00 => 0x2000,
    //         0b01 => 0x2400,
    //         0b10 => 0x2800,
    //         0b11 => 0x2c00,
    //         _ => panic!("impossible"),
    //     }
    // }

    pub fn background_pattern_addr(&self) -> u16 {
        match self.contains(Self::BACKGROUND_PATTERN_ADDR) {
            true => 0x1000,
            false => 0x0,
        }
    }

    pub fn sprite_pattern_addr(&self) -> u16 {
        match self.contains(ControlRegister::SPRITE_PATTERN_ADDR) {
            true => 0x1000,
            false => 0x0,
        }
    }

    pub fn sprite_size(&self) -> u8 {
        if !self.contains(ControlRegister::SPRITE_SIZE) {
            8
        } else {
            16
        }
    }

    pub fn update(&mut self, data: u8) {
        *self = ControlRegister::from_bits_truncate(data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_control_register() {
        let ctrl = ControlRegister::new();
        assert_eq!(ctrl.bits(), 0);
        assert_eq!(ctrl.addr_increment(), 1);
        assert_eq!(ctrl.generate_vblank_nmi(), false);
        assert_eq!(ctrl.background_pattern_addr(), 0x0000);
    }

    #[test]
    fn test_update_control_register() {
        let mut ctrl = ControlRegister::new();
        ctrl.update(0b10110000);
        assert!(ctrl.contains(ControlRegister::GENERATE_NMI));
        assert!(ctrl.contains(ControlRegister::SPRITE_SIZE));
        assert!(ctrl.contains(ControlRegister::BACKGROUND_PATTERN_ADDR));
        assert!(!ctrl.contains(ControlRegister::VRAM_ADD_INCREMENT));
    }

    #[test]
    fn test_sprite_size() {
        let mut ctrl = ControlRegister::new();
        ctrl.update(0b0010_0000);
        assert!(ctrl.contains(ControlRegister::SPRITE_SIZE));
        ctrl.update(0b0000_0000);
        assert_eq!(ctrl.contains(ControlRegister::SPRITE_SIZE), false);
    }

    #[test]
    fn test_vram_increment() {
        let mut ctrl = ControlRegister::new();
        assert_eq!(ctrl.addr_increment(), 1);

        ctrl.update(ControlRegister::VRAM_ADD_INCREMENT.bits());
        assert_eq!(ctrl.addr_increment(), 32);
    }

    #[test]
    fn test_nmi_generation() {
        let mut ctrl = ControlRegister::new();
        assert_eq!(ctrl.generate_vblank_nmi(), false);

        ctrl.update(ControlRegister::GENERATE_NMI.bits());
        assert_eq!(ctrl.generate_vblank_nmi(), true);
    }

    #[test]
    fn test_background_pattern_addr() {
        let mut ctrl = ControlRegister::new();
        assert_eq!(ctrl.background_pattern_addr(), 0x0000);

        ctrl.update(ControlRegister::BACKGROUND_PATTERN_ADDR.bits());
        assert_eq!(ctrl.background_pattern_addr(), 0x1000);
    }
}
