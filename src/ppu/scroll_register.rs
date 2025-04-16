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

/// Fields:
/// - `t`: Temporary VRAM address (15 bits). Used to compose scroll and addressing data.
/// - `v`: Current VRAM address (15 bits). Used for actual PPU memory reads/writes.
/// - `x`: Fine X scroll (3 bits). Controls horizontal pixel offset within a tile (0–7).
/// - `w`: First/second write toggle. Controls whether the next write to $2005/$2006 is the first or second.
///
/// Behavior:
/// - Writes to $2005 and $2006 update `t`, then `v` is copied from `t` during specific PPU steps.
/// - `x` is written via the first write to $2005 (fine X).
/// - `w` toggles after each write to $2005/$2006 to alternate between high/low or horizontal/vertical components.
pub struct ScrollRegister {
    pub v: u16, // Current VRAM address (15 bits)

    /// Temporary VRAM address (15 bits)
    ///
    /// ```text
    /// yyy NN YYYYY XXXXX
    /// ||| || ||||| +++++-- coarse X scroll (5 bits)
    /// ||| || +++++-------- coarse Y scroll (5 bits)
    /// ||| ++-------------- nametable select (2 bits)
    /// +++----------------- fine Y scroll (3 bits)
    /// ```
    pub t: u16,

    /// Fine X scroll (3 bits)
    pub x: u8,

    /// First/second write toggle
    pub w: bool,
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

    pub fn write_scroll(&mut self, data: u8) {
        if !self.w {
            // First write to $2005 — horizontal scroll
            self.x = data & 0b0000_0111; // fine X = bits 0–2
            let coarse_x = (data >> 3) as u16;

            // Set bits 0–4 (coarse X)
            self.t = (self.t & !0b00000_00000_11111) | (coarse_x & 0b1_1111);
        } else {
            // Second write to $2005 — vertical scroll
            let fine_y = (data & 0b0000_0111) as u16; // bits 0–2
            let coarse_y = ((data >> 3) & 0b1_1111) as u16; // bits 3–7

            // Preserve NT bits (10–11) and coarse X (0–4)
            self.t = (self.t & !0b0111_1111_1110_0000) // clears fine Y and coarse Y, preserves NT and coarse X
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

    pub fn increment_x(&mut self) {
        if (self.v & 0x001F) == 31 {
            self.v &= !0x001F;
            self.v ^= 0x0400;
        } else {
            self.v += 1;
        }
    }

    pub fn increment_y(&mut self) {
        // Increment fine Y
        if (self.v & 0x7000) != 0x7000 {
            self.v += 0x1000; // Increment fine Y
        } else {
            // Fine Y wraps around to 0
            self.v &= !0x7000; // Set fine Y to 0
            let mut y = (self.v >> 5) & 0x1F; // Get coarse Y

            // Increment coarse Y
            if y == 29 {
                y = 0; // Wrap around coarse Y
                self.v ^= 0x0800; // Toggle vertical nametable
            } else if y == 31 {
                y = 0; // Wrap around coarse Y without affecting nametable
            } else {
                y += 1; // Increment coarse Y
            }

            // Update coarse Y in the v register
            self.v = (self.v & !0x03E0) | (y << 5);
        }
    }

    pub fn copy_horizontal_bits(&mut self) {
        // Mask bits: 0000010000011111 -> NT X + Coarse X
        let mask = 0b0000010000011111;

        // Copy NT X and coarse X (bits 10 and 0-4)
        self.v &= !mask;
        self.v |= self.t & mask;
    }

    pub fn copy_vertical_bits(&mut self) {
        // Mask bits: 0111101111100000 -> Fine Y + NT Y + Coarse Y
        let mask = 0b0111101111100000;

        // Copy fine Y, coarse Y, and NT Y (bits 12-5 and bit 11)
        self.v &= !mask;
        self.v |= self.t & mask;
    }

    // pub fn coarse_x(&self) -> u8 {
    //     (self.v & 0b11111) as u8
    // }
    // pub fn coarse_y(&self) -> u8 {
    //     ((self.v >> 5) & 0b11111) as u8
    // }
    // pub fn fine_y(&self) -> u8 {
    //     ((self.v >> 12) & 0b111) as u8
    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Decompose 15-bit v register into its parts
    fn print_decoded_v(v: u16) {
        println!("v = 0x{:04X} ({})", v, v);

        let fine_y = (v >> 12) & 0b111;
        let nt_y = (v >> 11) & 0b1;
        let nt_x = (v >> 10) & 0b1;
        let coarse_y = (v >> 5) & 0b1_1111;
        let coarse_x = v & 0b1_1111;

        println!("Fine Y    (bits 12–14): {:03b} ({})", fine_y, fine_y);
        println!("Nametable Y (bit 11)  : {}", nt_y);
        println!("Nametable X (bit 10)  : {}", nt_x);
        println!("Coarse Y  (bits 5–9)  : {:05b} ({})", coarse_y, coarse_y);
        println!("Coarse X  (bits 0–4)  : {:05b} ({})", coarse_x, coarse_x);
    }

    #[test]
    fn test_horizontal_scroll_write() {
        let mut sr = ScrollRegister::new();

        sr.write_scroll(0b10101_011); // coarse_x = 0b10101 = 21, fine_x = 0b011 = 3
        assert_eq!(sr.x, 0b011);
        assert_eq!(sr.t & 0b11111, 0b10101); // coarse X
        assert_eq!(sr.w, true);
    }

    // #[test]
    // fn test_vertical_scroll_write() {
    //     let mut sr = ScrollRegister::new();
    //     sr.w = true; // simulate first write already happened
    //
    //     sr.t = 0b0000_1100_0001_1111; // preset nametable bits = 0b11
    //     sr.write_scroll(0b11010_101); // coarse_y = 0b11010 = 26, fine_y = 0b101 = 5
    //
    //     assert_eq!(sr.t & 0b0111_0000_0000_0000, 0b101 << 12); // fine Y
    //     assert_eq!(sr.t & 0b0000_0011_1110_0000, 26 << 5);     // coarse Y
    //     assert_eq!(sr.t & 0b0000_1100_0000_0000, 0b1100_0000_0000); // nametable bits preserved
    //     assert_eq!(sr.w, false);
    // }

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

    #[test]
    fn test_scroll_register_address_wrapping() {
        let mut sr = ScrollRegister::new();
        sr.write_to_addr(0x2C); // High byte of address
        sr.write_to_addr(0x00); // Low byte

        assert_eq!(sr.v, 0x2C00);

        sr.increment_x(); // coarse_x++
        assert_eq!(sr.v, 0x2C01);

        print_decoded_v(sr.v);
        sr.increment_y(); // fine_y++
        print_decoded_v(sr.v);
        let got = sr.v;
        let want = 0x3C01;
        assert_eq!(
            sr.v,
            want,
            "{}",
            format!("\n\tgot:  {:016b}\n\twant: {:016b}", got, want)
        ); // ✅
    }

    #[test]
    fn test_scroll_register_fine_y_wraps_and_coarse_y_increments() {
        let mut sr = ScrollRegister::new();
        sr.write_to_addr(0x2C); // High byte
        sr.write_to_addr(0x00); // Low byte
        assert_eq!(sr.v, 0x2C00);

        // Set fine_y = 7 (bits 12–14), coarse_y = 4 (bits 5–9)
        sr.v |= (7 << 12) | (4 << 5);
        print_decoded_v(sr.v);

        sr.increment_y(); // should wrap fine_y to 0 and increment coarse_y (4 → 5)
        print_decoded_v(sr.v);

        let got = sr.v;
        let want = 0x0CA0; // fine_y = 0, coarse_y = 5, other bits unchanged
        assert_eq!(
            got,
            want,
            "{}",
            format!("\n\tgot:  {:016b}\n\twant: {:016b}", got, want)
        );
    }
}
