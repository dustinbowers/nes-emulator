mod common;

use nes_emulator::nes::NES;

fn create_nes_emulator() -> NES {
    let nes = NES::new();
}

#[test]
fn test_vbl_is_set() {

}