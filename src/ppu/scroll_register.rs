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

            // Clear fine Y (bits 12–14) and coarse Y (5–9), keep everything else
            self.t &= !(0b111 << 12); // clear fine Y
            self.t &= !(0b11111 << 5); // clear coarse Y
            self.t |= coarse_y << 5;
            self.t |= fine_y << 12;
        }
        self.w = !self.w;
    }

    pub fn write_to_addr(&mut self, data: u8) {
        if !self.w {
            // First write (high byte of address)
            // self.t = (self.t & 0x00FF) | (((data as u16) & 0x3F) << 8);
            self.t &= 0x00FF;
            self.t |= (data as u16 & 0x3F) << 8;
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
            // if coarse_x == 31, wrap to 0
            self.v &= !0x001F; // clear coarse X (bits 0–4)
            self.v ^= 0x0400; // toggle horizontal nametable select (bit 10)
        } else {
            // coarse_x < 31, just increment
            self.v += 1;
        }
    }

    pub fn increment_y(&mut self) {
        // println!(">> increment_y before: v = {:04X}, fine Y = {:01X}", self.v, (self.v >> 12) & 0b111);

        // Fine Y is bits 12–14
        if (self.v & 0x7000) != 0x7000 {
            self.v += 0x1000; // increment fine Y
        } else {
            self.v &= !0x7000; // fine Y = 0
            let mut y = (self.v >> 5) & 0x1F; // extract coarse Y (5 bits)

            if y == 29 {
                y = 0;
                self.v ^= 0x0800; // switch vertical nametable
            } else if y == 31 {
                y = 0; // stay in same nametable
            } else {
                y += 1;
            }

            self.v = (self.v & !0x03E0) | (y << 5); // put coarse Y back into v
        }
    }

    pub fn copy_horizontal_bits(&mut self) {
        // bit 10 (nametable X) + bits 4-0 (coarse X)
        let mask = 0b00000_1_00000_11111;

        // Copy NT X and coarse X (bits 10 and 0-4)
        self.v &= !mask;
        self.v |= self.t & mask;
    }

    pub fn copy_vertical_bits(&mut self) {
        // bit 11 (nametable Y), bits 9–5 (coarse Y), and bits 12–14 (fine Y)
        let mask = 0b0111_10_11111_00000;

        // Copy fine Y, coarse Y, and NT Y (bits 12-5 and bit 11)
        self.v &= !mask;
        self.v |= self.t & mask;
    }
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

    #[test]
    fn test_copy_vertical_bits_preserves_horizontal() {
        let mut sr = ScrollRegister::new();

        // v has arbitrary horizontal values (coarse_x = 15, nametable_x = 1)
        sr.v = 0b11111 | (1 << 10);
        // t has vertical values to copy (fine_y = 3, coarse_y = 20, nametable_y = 1)
        sr.t = (3 << 12) | (20 << 5) | (1 << 11);

        sr.copy_vertical_bits();

        // Fine Y (bits 12-14) = 3
        assert_eq!((sr.v >> 12) & 0b111, 3);
        // Coarse Y (bits 5-9) = 20
        assert_eq!((sr.v >> 5) & 0b1_1111, 20);
        // Nametable Y (bit 11) = 1
        assert_eq!((sr.v >> 11) & 1, 1);

        // Ensure horizontal bits were untouched
        assert_eq!(sr.v & 0b11111, 0b11111); // coarse X
        assert_eq!((sr.v >> 10) & 1, 1); // nametable X
    }

    #[test]
    fn test_copy_vertical_bits_zero_values() {
        let mut sr = ScrollRegister::new();
        sr.v = 0xFFFF;
        sr.t = 0x0000;

        sr.copy_vertical_bits();

        assert_eq!((sr.v >> 12) & 0b111, 0); // fine Y
        assert_eq!((sr.v >> 5) & 0b1_1111, 0); // coarse Y
        assert_eq!((sr.v >> 11) & 1, 0); // nametable Y
    }

    #[test]
    fn test_copy_vertical_bits_all_vertical_bits_set() {
        let mut sr = ScrollRegister::new();

        // Set t to all 1s in the vertical fields
        sr.t = (0b111 << 12) | (0b1_1111 << 5) | (1 << 11);
        sr.v = 0; // clear v

        sr.copy_vertical_bits();

        assert_eq!((sr.v >> 12) & 0b111, 0b111); // fine Y
        assert_eq!((sr.v >> 5) & 0b1_1111, 0b1_1111); // coarse Y
        assert_eq!((sr.v >> 11) & 1, 1); // nametable Y
    }

    #[test]
    fn test_copy_vertical_bits_partial_overwrite() {
        let mut sr = ScrollRegister::new();

        // Set some initial v value
        sr.v = 0b01011_1111_0110_0000;
        // t only has fine_y = 2, others zero
        sr.t = 0b010 << 12;

        sr.copy_vertical_bits();

        assert_eq!((sr.v >> 12) & 0b111, 0b010); // fine Y copied
        assert_eq!((sr.v >> 5) & 0b1_1111, 0); // coarse Y cleared
        assert_eq!((sr.v >> 11) & 1, 0); // nametable Y cleared
    }
}
