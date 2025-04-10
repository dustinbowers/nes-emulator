use crate::cartridge::nrom::NromCart;
use crate::cartridge::Cartridge;

const NES_MAGIC_BYTES: &[u8; 4] = b"NES\x1A";
const PRG_ROM_PAGE_SIZE: usize = 0x4000;
const CHR_ROM_PAGE_SIZE: usize = 0x2000;

#[derive(Debug)]
pub enum RomError {
    InvalidFormat(String),
    UnsupportedVersion(String),
    OutOfBounds(String),
}

#[derive(Clone, Debug)]
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
    pub fn new(raw: &Vec<u8>) -> Result<Rom, RomError> {
        // Check NES magic bytes
        if raw.get(0..4) != Some(NES_MAGIC_BYTES) {
            return Err(RomError::InvalidFormat(
                "File is not iNES file format".to_string(),
            ));
        }

        // Extract mapper information
        let mapper = (raw[7] & 0b1111_0000) | (raw[6] >> 4);

        // Check iNES version
        let ines_ver = (raw[7] >> 2) & 0b11;
        if ines_ver != 0 {
            return Err(RomError::UnsupportedVersion(
                "NES2.0 format is not supported".to_string(),
            ));
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

    pub fn into_cartridge(self) -> Box<dyn Cartridge> {
        match self.mapper {
            0 => {
                let chr_rom_len = self.chr_rom.len();
                println!("rom::into_cartridge() - chr_rom_len = {}", chr_rom_len);
                let mut cart = NromCart::new(self.prg_rom, self.chr_rom, self.screen_mirroring);
                if chr_rom_len == 0 {
                    println!("Nrom, setting chr_is_ram = true");
                    cart.chr_is_ram = true;
                }
                Box::new(cart)
            }

            // TODO
            id => panic!("Unsupported mapper id: {}", id),
        }
    }
}

impl Into<Box<dyn Cartridge>> for Rom {
    fn into(self) -> Box<dyn Cartridge> {
        self.into_cartridge()
    }
}
