pub enum OamDmaOp {
    Dummy,
    Read(u16),
    Write,
}

pub struct OamDma {
    active: bool,
    page: u8,
    cycle: u16,
    latch: u8,
    dummy_cycles: u8,
}

impl OamDma {
    pub fn new() -> Self {
        Self {
            active: false,
            page: 0,
            cycle: 0,
            latch: 0,
            dummy_cycles: 0,
        }
    }

    pub fn active(&self) -> bool {
        self.active
    }

    /// Start OAM DMA. `cpu_odd_cycle` decides whether a dummy cycle is needed
    pub fn start(&mut self, page: u8, cpu_odd_cycle: bool) {
        self.active = true;
        self.page = page;
        self.cycle = 0;
        self.dummy_cycles = if cpu_odd_cycle { 2 } else { 1 };
    }

    pub fn step(&mut self) -> OamDmaOp {
        if !self.active {
            return OamDmaOp::Dummy;
        }

        if self.dummy_cycles > 0 {
            self.dummy_cycles -= 1;
            return OamDmaOp::Dummy;
        }

        // 512 cycles to complete 256 read/write pairs
        let phase = self.cycle & 1;
        let index = self.cycle >> 1;

        let op = if phase == 0 {
            let addr = ((self.page as u16) << 8) | index;
            OamDmaOp::Read(addr)
        } else {
            OamDmaOp::Write
        };

        self.cycle += 1;

        // done when 256 bytes have been written
        if index == 255 && phase == 1 {
            self.active = false;
        }

        op
    }
}
