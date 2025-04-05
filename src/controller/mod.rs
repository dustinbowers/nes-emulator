pub mod joypad;

pub trait NesController {
    fn read(&mut self) -> u8;
    fn write(&mut self, data: u8);
}
