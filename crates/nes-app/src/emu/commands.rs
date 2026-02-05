use nes_core::nes::cartridge;

pub enum EmuCommand {
    InsertCartridge(Box<dyn cartridge::Cartridge>),
    Reset,
    Pause(bool),
}
