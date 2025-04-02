pub struct AddressRegister {
    value: (u8, u8),
    hi_ptr: bool,
}

impl AddressRegister {
    pub fn new() -> Self {
        AddressRegister {
            value: (0, 0), // (hi-byte, lo-byte)
            hi_ptr: true,
        }
    }
    pub fn set(&mut self, data: u16) {
        self.value.0 = (data >> 8) as u8;
        self.value.1 = (data & 0xff) as u8;
    }

    pub fn update(&mut self, data: u8) {
        // println!("address_register.update(${:02X}) - hi_ptr = {}, current_address = {:04X}", data, self.hi_ptr, self.get());
        if self.hi_ptr {
            self.value.0 = data;
        } else {
            self.value.1 = data;
        }
        self.hi_ptr = !self.hi_ptr;
    }

    pub fn increment(&mut self, inc: u8) {
        let lo = self.value.1;
        self.value.1 = lo.wrapping_add(inc);

        if lo.wrapping_add(inc) < lo {
            self.value.0 = self.value.0.wrapping_add(1);
        }
        if self.get() > 0x3fff {
            self.set(self.get() & 0b11_1111_1111_1111); // mirror down if above 0x3FFF
        }
    }

    pub fn reset_latch(&mut self) {
        self.hi_ptr = true;
    }

    pub fn get(&self) -> u16 {
        let addr = ((self.value.0 as u16) << 8) | (self.value.1 as u16);
        addr // TOOD: handle mirroring elsewhere
        // addr & 0x3FFF // Mirror address down to $0000-$3FFF range
    }
}

#[cfg(test)]
mod tests {
    use super::AddressRegister;

    #[test]
    fn test_initial_state() {
        let addr = AddressRegister::new();
        assert_eq!(addr.get(), 0);
    }

    #[test]
    fn test_update_high_byte_first() {
        let mut addr = AddressRegister::new();
        addr.update(0x12); // High byte
        addr.update(0x34); // Low byte
        assert_eq!(addr.get(), 0x1234);
    }

    #[test]
    fn test_update_resets_latch() {
        let mut addr = AddressRegister::new();
        addr.update(0xAB); // write hi-byte
        addr.update(0xCD); // write lo-byte
        addr.update(0xEF); // Should overwrite old hi-byte
        let want = 0xEFCD;
        let got = addr.get();
        assert_eq!(want, got, "{}",
        format!("Want: {:04X}, Got: {:04X}", want, got));
    }

    #[test]
    fn test_increment_within_same_page() {
        let mut addr = AddressRegister::new();
        addr.set(0x1234);
        addr.increment(1);
        assert_eq!(addr.get(), 0x1235);
    }

    #[test]
    fn test_increment_causes_page_carry() {
        let mut addr = AddressRegister::new();
        addr.set(0x12FF);
        addr.increment(1);
        assert_eq!(addr.get(), 0x1300);
    }

    #[test]
    fn test_increment_wraps_properly() {
        let mut addr = AddressRegister::new();
        addr.set(0x3FFF);
        addr.increment(1);
        assert_eq!(addr.get(), 0x0000);
    }

    #[test]
    fn test_reset_latch() {
        let mut addr = AddressRegister::new();
        addr.update(0x12);
        addr.reset_latch();
        addr.update(0x34);
        assert_eq!(addr.get(), 0x3400);
    }
}

