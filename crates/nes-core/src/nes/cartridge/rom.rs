use crate::nes::cartridge::Cartridge;
use crate::nes::cartridge::mapper000_nrom::NromCart;
use crate::nes::cartridge::mapper001_mmc1::Mmc1;
use crate::nes::cartridge::mapper002_ux_rom::Mapper002UxRom;
use crate::nes::cartridge::mapper003_cn_rom::Mapper003CnRom;
use crate::nes::cartridge::mapper004_mmc3::Mmc3;
use thiserror::Error;

const NES_MAGIC_BYTES: &[u8; 4] = b"NES\x1A";
const PRG_ROM_PAGE_SIZE: usize = 0x4000;
const CHR_ROM_PAGE_SIZE: usize = 0x2000;

#[derive(Debug, Error)]
pub enum RomError {
    #[error("{0}")]
    InvalidFormat(String),

    #[error("Unsupported ROM version: v{0}")]
    UnsupportedVersion(u8),

    #[error("Unsupported Mapper: {0}")]
    UnsupportedMapper(u8),
}

#[derive(Copy, Clone, Debug)]
pub enum Mirroring {
    Vertical,
    Horizontal,
    FourScreen,
    Single0,
    Single1,
}

pub struct Rom {
    pub prg_rom: Vec<u8>,
    pub chr_rom: Vec<u8>,
    pub mapper: u8,
    pub screen_mirroring: Mirroring,
}

impl Rom {
    pub fn parse(raw: &Vec<u8>) -> Result<Rom, RomError> {
        // Check NES magic bytes
        if raw.len() < 16 || &raw[0..4] != NES_MAGIC_BYTES {
            return Err(RomError::InvalidFormat("Not an iNES file".into()));
        }

        // Extract mapper information
        let mapper = (raw[7] & 0b1111_0000) | (raw[6] >> 4);

        // Check iNES version
        let ines_ver = (raw[7] >> 2) & 0b11;
        if ines_ver != 0 {
            return Err(RomError::UnsupportedVersion(2));
        }

        // Determine mirroring type
        let four_screen = raw[6] & 0b1000 != 0;
        let vertical_mirroring = raw[6] & 0b1 != 0;
        let screen_mirroring = match (four_screen, vertical_mirroring) {
            (true, _) => Mirroring::FourScreen,
            (false, true) => Mirroring::Vertical,
            (false, false) => Mirroring::Horizontal,
        };

        let prg_rom_size = raw[4] as usize * PRG_ROM_PAGE_SIZE;
        let chr_rom_size = raw[5] as usize * CHR_ROM_PAGE_SIZE;

        let skip_trainer = raw[6] & 0b100 != 0;

        let prg_rom_start = 16 + if skip_trainer { 512 } else { 0 };
        let chr_rom_start = prg_rom_start + prg_rom_size;

        Ok(Rom {
            prg_rom: raw[prg_rom_start..(prg_rom_start + prg_rom_size)].to_vec(),
            chr_rom: raw[chr_rom_start..(chr_rom_start + chr_rom_size)].to_vec(),
            mapper,
            screen_mirroring,
        })
    }

    #[cfg(test)]
    pub fn empty() -> Rom {
        Self::parse(&vec![]).unwrap()
    }

    #[cfg(test)]
    pub fn new_custom(
        prg_rom: Vec<u8>,
        chr_rom: Vec<u8>,
        mapper: u8,
        screen_mirroring: Mirroring,
    ) -> Rom {
        Rom {
            prg_rom,
            chr_rom,
            mapper,
            screen_mirroring,
        }
    }

    pub fn into_cartridge(self) -> Result<Box<dyn Cartridge>, RomError> {
        match self.mapper {
            0 => {
                let chr_rom_len = self.chr_rom.len();
                let mut cart = NromCart::new(self.prg_rom, self.chr_rom, self.screen_mirroring);
                if chr_rom_len == 0 {
                    cart.chr_is_ram = true;
                }
                Ok(Box::new(cart))
            }
            1 => {
                let cart = Mmc1::new(self.prg_rom, self.chr_rom, 0x2000);
                Ok(Box::new(cart))
            }
            2 => {
                let cart = Mapper002UxRom::new(self.prg_rom, self.chr_rom, self.screen_mirroring);
                Ok(Box::new(cart))
            }
            3 => {
                let cart = Mapper003CnRom::new(self.prg_rom, self.chr_rom, self.screen_mirroring);
                Ok(Box::new(cart))
            }
            4 => {
                let cart = Mmc3::new(self.prg_rom, self.chr_rom, self.screen_mirroring);
                Ok(Box::new(cart))
            }

            // TODO
            id => Err(RomError::UnsupportedMapper(id)),
        }
    }
}
