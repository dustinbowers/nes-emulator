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
    pub v: u16, // Current VRAM address (15 bits)
    pub t: u16, // Temporary VRAM address (15 bits)
    pub x: u8,  // Fine X scroll (3 bits)
    pub w: bool // First/second write toggle
}

impl ScrollRegister {
    pub fn new() -> Self {
        Self {
            v: 0,
            t: 0,
            x: 0,
            w: false,
        }
    }

    pub fn write(&mut self, data: u8) {
        if !self.w {
            // First write to $2005 — horizontal scroll
            self.x = data & 0b0000_0111; // fine X = bits 0–2
            let coarse_x = (data >> 3) as u16;

            // Set bits 0–4 (coarse X)
            self.t = (self.t & !0b00000_00000_11111) | (coarse_x & 0b1_1111);
        } else {
            // Second write to $2005 — vertical scroll
            let fine_y = (data & 0b0000_0111) as u16;       // bits 0–2
            let coarse_y = ((data >> 3) & 0b1_1111) as u16; // bits 3–7

            // Clear bits 5–9 (coarse Y) and 12–14 (fine Y), preserve 10–11 (nametable bits)
            self.t = (self.t & !0b1110_0000_0111_0000)
                | (coarse_y << 5)
                | (fine_y << 12);
        }

        self.w = !self.w;
    }

    pub fn write_to_addr(&mut self, data: u8) {
        if !self.w {
            // First write (high byte of address)
            self.t = (self.t & 0x00FF) | (((data as u16) & 0x3F) << 8);
        } else {
            // Second write (low byte of address)
            self.t = (self.t & 0xFF00) | (data as u16);
            self.v = self.t;
        }
        self.w = !self.w;
    }

    pub fn get_addr(&self) -> u16 {
        self.v & 0x3FFF // mirror down to 0x0000–0x3FFF
    }

    pub fn increment_addr(&mut self, inc: u8) {
        self.v = self.v.wrapping_add(inc as u16);
    }

    pub fn reset_latch(&mut self) {
        self.w = false;
    }


    pub(crate) fn increment_x(&mut self) {
        if (self.v & 0x001F) == 31 {
            self.v &= !0x001F;
            self.v ^= 0x0400;
        } else {
            self.v += 1;
        }
    }

    pub(crate) fn increment_y(&mut self) {
        let mut v = self.v;
        if (v & 0x7000) != 0x7000 {
            v += 0x1000; // increment fine Y
        } else {
            v &= !0x7000; // fine Y = 0
            let mut y = (v >> 5) & 0x1F;
            if y == 29 {
                y = 0;
                v ^= 0x0800; // switch vertical nametable
            } else if y == 31 {
                y = 0;
            } else {
                y += 1;
            }
            v = (v & !0x03E0) | (y << 5);
        }
        self.v = v;
    }

    pub(crate) fn copy_horizontal_bits(&mut self) {
        // v: .....F.. ...EDCBA = t: .....F.. ...EDCBA
        // Copy NT X and coarse X (bits 10 and 0-4)
        self.v &= !0b0000010000011111;
        self.v |= self.t & 0b0000010000011111;
    }

    pub(crate) fn copy_vertical_bits(&mut self) {
        // v: .IHGF.ED CBA..... = t: .IHGF.ED CBA.....
        // Copy fine Y, coarse Y, and NT Y (bits 12-5 and bit 11)
        self.v &= !0b0111101111100000;
        self.v |= self.t & 0b0111101111100000;
    }

    pub fn coarse_x(&self) -> u8 {
        (self.v & 0b11111) as u8
    }
    pub fn coarse_y(&self) -> u8 {
        ((self.v >> 5) & 0b11111) as u8
    }
    pub fn fine_y(&self) -> u8 {
        ((self.v >> 12) & 0b111) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_horizontal_scroll_write() {
        let mut sr = ScrollRegister::new();

        sr.write(0b10101_011); // coarse_x = 0b10101 = 21, fine_x = 0b011 = 3
        assert_eq!(sr.x, 0b011);
        assert_eq!(sr.t & 0b11111, 0b10101); // coarse X
        assert_eq!(sr.w, true);
    }

    #[test]
    fn test_vertical_scroll_write() {
        let mut sr = ScrollRegister::new();
        sr.w = true; // simulate first write already happened

        sr.t = 0b0000_1100_0001_1111; // preset nametable bits = 0b11
        sr.write(0b11010_101); // coarse_y = 0b11010 = 26, fine_y = 0b101 = 5

        assert_eq!(sr.t & 0b0111_0000_0000_0000, 0b101 << 12); // fine Y
        assert_eq!(sr.t & 0b0000_0011_1110_0000, 26 << 5);     // coarse Y
        assert_eq!(sr.t & 0b0000_1100_0000_0000, 0b1100_0000_0000); // nametable bits preserved
        assert_eq!(sr.w, false);
    }

    #[test]
    fn test_write_to_addr() {
        let mut sr = ScrollRegister::new();

        sr.write_to_addr(0x2C); // high byte (only 6 bits used)
        assert_eq!(sr.t, 0x2C00);
        assert_eq!(sr.w, true);

        sr.write_to_addr(0x10); // low byte
        assert_eq!(sr.t, 0x2C10);
        assert_eq!(sr.v, 0x2C10);
        assert_eq!(sr.w, false);
    }

    #[test]
    fn test_increment_addr_wraparound() {
        let mut sr = ScrollRegister::new();
        sr.v = 0x3FFF;
        sr.increment_addr(1);
        assert_eq!(sr.get_addr(), 0x0000); // wraps at 0x4000
    }

    #[test]
    fn test_get_addr_masks_to_0x3fff() {
        let mut sr = ScrollRegister::new();
        sr.v = 0x7FFF;
        assert_eq!(sr.get_addr(), 0x3FFF);
    }

    #[test]
    fn test_increment_x_normal() {
        let mut sr = ScrollRegister::new();

        // Set coarse_x = 10
        sr.v = 0b01010;

        sr.increment_x();

        let coarse_x = sr.v & 0b11111;
        assert_eq!(coarse_x, 11);
    }

    #[test]
    fn test_increment_x_from_zero() {
        let mut sr = ScrollRegister::new();

        sr.v = 0b00000;

        sr.increment_x();

        assert_eq!(sr.v & 0b11111, 1);
    }

    #[test]
    fn test_increment_x_wrap_and_toggle_nametable() {
        let mut sr = ScrollRegister::new();

        // Set coarse_x = 31, h-nametable = 0
        sr.v = 0b11111 | (0 << 10);

        sr.increment_x();

        let coarse_x = sr.v & 0b11111;
        let h_nametable = (sr.v >> 10) & 1;

        assert_eq!(coarse_x, 0);
        assert_eq!(h_nametable, 1); // toggled
    }

    #[test]
    fn test_increment_x_wrap_with_existing_nametable_toggle() {
        let mut sr = ScrollRegister::new();

        // coarse_x = 31, h-nametable = 1
        sr.v = 0b11111 | (1 << 10);

        sr.increment_x();

        let coarse_x = sr.v & 0b11111;
        let h_nametable = (sr.v >> 10) & 1;

        assert_eq!(coarse_x, 0);
        assert_eq!(h_nametable, 0); // toggled back
    }


    #[test]
    fn test_increment_y_fine_y() {
        let mut sr = ScrollRegister::new();

        // Set v with fine_y = 5
        sr.v = 0b101 << 12;

        sr.increment_y();

        let fine_y = (sr.v >> 12) & 0b111;
        assert_eq!(fine_y, 6);
    }

    #[test]
    fn test_increment_y_rolls_fine_y_and_increments_coarse_y() {
        let mut sr = ScrollRegister::new();

        // Set v with fine_y = 7, coarse_y = 15
        sr.v = (7 << 12) | (15 << 5);

        sr.increment_y();

        let fine_y = (sr.v >> 12) & 0b111;
        let coarse_y = (sr.v >> 5) & 0b11111;

        assert_eq!(fine_y, 0);
        assert_eq!(coarse_y, 16);
    }

    #[test]
    fn test_increment_y_toggles_nametable_on_coarse_y_29() {
        let mut sr = ScrollRegister::new();

        // fine_y = 7, coarse_y = 29, vnametable bit = 0
        sr.v = (7 << 12) | (29 << 5);

        sr.increment_y();

        let coarse_y = (sr.v >> 5) & 0b11111;
        let v_nametable_bit = (sr.v >> 11) & 1;

        assert_eq!(coarse_y, 0);
        assert_eq!(v_nametable_bit, 1);
    }

    #[test]
    fn test_increment_y_wraps_coarse_y_31_to_0_no_toggle() {
        let mut sr = ScrollRegister::new();

        // fine_y = 7, coarse_y = 31, vnametable bit = 1
        sr.v = (7 << 12) | (31 << 5) | (1 << 11);

        sr.increment_y();

        let coarse_y = (sr.v >> 5) & 0b11111;
        let v_nametable_bit = (sr.v >> 11) & 1;

        assert_eq!(coarse_y, 0);
        assert_eq!(v_nametable_bit, 1); // unchanged
    }

}
