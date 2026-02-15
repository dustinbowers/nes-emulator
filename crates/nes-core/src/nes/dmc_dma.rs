pub struct DmcDma {
    // request address latched from the DMC channel
    req_addr: Option<u16>,

    // active transfer state
    active: bool,
    cycles_left: u8, // counts down CPU cycles remaining in the stall
    active_addr: u16,
}

impl DmcDma {
    pub fn new() -> Self {
        Self {
            req_addr: None,
            active: false,
            cycles_left: 0,
            active_addr: 0,
        }
    }

    pub fn request(&mut self, addr: u16) {
        if self.req_addr.is_none() && !self.active {
            self.req_addr = Some(addr);
        }
    }

    pub fn pending(&self) -> bool {
        self.req_addr.is_some()
    }

    pub fn active(&self) -> bool {
        self.active
    }

    /// Start the DMA stall
    pub fn begin(&mut self) {
        if !self.active {
            if let Some(addr) = self.req_addr.take() {
                self.active = true;
                self.active_addr = addr;
                self.cycles_left = 4; // common emulation choice
            }
        }
    }

    /// Called once per CPU cycle while DMC owns the bus.
    /// Returns Some(addr) when a memory read should happen
    pub fn step(&mut self) -> Option<u16> {
        if !self.active {
            return None;
        }

        // Count down one stolen CPU cycle
        self.cycles_left -= 1;

        // Do the read on the last stolen cycle
        let do_read_now = self.cycles_left == 0;
        if do_read_now {
            self.active = false;
            return Some(self.active_addr);
        }

        None
    }
}
