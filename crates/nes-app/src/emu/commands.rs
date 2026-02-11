use nes_core::nes::cartridge;

pub enum AudioChannel {
    Pulse1,
    Pulse2,
    Triangle,
    Noise,
    DMC,
}
pub enum EmuCommand {
    InsertCartridge(Box<dyn cartridge::Cartridge>),
    Reset,
    Pause(bool),

    ToggleAudioChannel(AudioChannel),
}
