/* See: https://www.nesdev.org/wiki/PPU_registers#PPUSCROLL
   1st write
   7  bit  0
   ---- ----
   XXXX XXXX
   |||| ||||
   ++++-++++- X scroll bits 7-0 (bit 8 in PPUCTRL bit 0)

   2nd write
   7  bit  0
   ---- ----
   YYYY YYYY
   |||| ||||
   ++++-++++- Y scroll bits 7-0 (bit 8 in PPUCTRL bit 1)
*/
pub struct ScrollRegister {
    pub scroll_x: u8,
    pub scroll_y: u8,
    pub write_latch: bool,
}

impl ScrollRegister {
    pub fn new() -> Self {
        ScrollRegister {
            scroll_x: 0,
            scroll_y: 0,
            write_latch: false,
        }
    }

    pub fn write(&mut self, data: u8) {
        if !self.write_latch {
            self.scroll_x = data;
        } else {
            self.scroll_y = data;
        }
        self.write_latch = !self.write_latch;
    }

    pub fn reset_latch(&mut self) {
        self.write_latch = false;
    }
}
