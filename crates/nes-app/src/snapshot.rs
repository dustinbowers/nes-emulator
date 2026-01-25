#[derive(Clone)]
pub struct CpuSnapshot {
    pub program_counter: u16,
    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub stack_pointer: u8,
    pub status: u8,
}

#[derive(Default, Clone)]
pub struct ApuSnapshot {
    pub mute_pulse1: bool,
    pub mute_pulse2: bool,
    pub mute_triangle: bool,
    pub mute_noise: bool,
    pub mute_dmc: bool,
}

#[derive(Clone)]
pub struct DebugSnapshot {
    pub cpu: CpuSnapshot,
}

#[derive(Clone)]
pub struct NesSnapshot {
    pub cpu: CpuSnapshot,
    pub apu: ApuSnapshot,
}

#[derive(Clone)]
pub struct FrameSnapshot {
    pub pixels: Vec<u8>,
}
