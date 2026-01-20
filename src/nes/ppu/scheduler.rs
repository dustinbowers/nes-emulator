pub const DOTS: usize = 341;
pub const SCAN_LINES: usize = 262;

pub static PPU_SCHEDULE: [[DotOperations; DOTS]; SCAN_LINES] = build_schedule();

/// Mask for operations that require rendering enabled
pub const RENDER_OPS: u64 = bit(PpuOperation::ShiftRegisters)
    | bit(PpuOperation::FetchNameTable)
    | bit(PpuOperation::FetchAttribute)
    | bit(PpuOperation::FetchTileLow)
    | bit(PpuOperation::FetchTileHigh)
    | bit(PpuOperation::RenderPixel)
    | bit(PpuOperation::IncCoarseX)
    | bit(PpuOperation::IncFineY)
    | bit(PpuOperation::CopyHorizV)
    | bit(PpuOperation::CopyVertV)
    | bit(PpuOperation::ClearSecondaryOam)
    | bit(PpuOperation::ResetSpriteEvaluation)
    | bit(PpuOperation::EvaluateSprites);

pub const fn bit(op: PpuOperation) -> u64 {
    1u64 << (op as u8)
}
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum PpuOperation {
    FetchNameTable,
    FetchAttribute,
    FetchTileLow,
    FetchTileHigh,
    LoadBackgroundRegisters,

    IncCoarseX,
    IncFineY,
    CopyHorizV,
    CopyVertV,

    ShiftRegisters,
    ClearSecondaryOam,

    FillSpriteRegister,
    EvaluateSprites,
    ResetSpriteEvaluation,

    RenderPixel,

    SetVBlank,
    ClearVBlank,

    None,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct DotOperations {
    pub len: u8,
    pub ops: [PpuOperation; 6],
}

impl DotOperations {
    pub const fn new() -> Self {
        Self {
            len: 0,
            ops: [PpuOperation::None; 6],
        }
    }

    pub const fn push(mut self, op: PpuOperation) -> Self {
        debug_assert!(self.len < 6);
        self.ops[self.len as usize] = op;
        self.len += 1;
        self
    }
}

const fn build_schedule() -> [[DotOperations; DOTS]; SCAN_LINES] {
    let mut table = [[DotOperations::new(); DOTS]; SCAN_LINES];
    let mut scanline = 0;
    while scanline < SCAN_LINES {
        let mut dot = 0;
        while dot < DOTS {
            table[scanline][dot] = schedule_for(scanline, dot);
            dot += 1;
        }
        scanline += 1;
    }
    table
}

/// Return a set of operations for a given scanline/dot combination
const fn schedule_for(scanline: usize, dot: usize) -> DotOperations {
    let mut ops = DotOperations::new();

    let visible = scanline < 240;
    let prerender = scanline == 261;
    let render_cycle = dot >= 1 && dot <= 256;
    let fetch_cycle = render_cycle || (dot >= 321 && dot <= 336);

    // Background fetch pipeline
    if (visible || prerender) && fetch_cycle {
        // shift registers every fetch
        ops = ops.push(PpuOperation::ShiftRegisters);

        match dot % 8 {
            1 => ops = ops.push(PpuOperation::FetchNameTable),
            3 => ops = ops.push(PpuOperation::FetchAttribute),
            5 => ops = ops.push(PpuOperation::FetchTileLow),
            7 => ops = ops.push(PpuOperation::FetchTileHigh),
            0 => {
                ops = ops.push(PpuOperation::LoadBackgroundRegisters);
                ops = ops.push(PpuOperation::IncCoarseX);
            }
            _ => {}
        }
    }

    // Render a pixel (dots 1-256)
    if visible && render_cycle {
        ops = ops.push(PpuOperation::RenderPixel);
    }

    // Fine Y increments
    if visible && dot == 256 {
        ops = ops.push(PpuOperation::IncFineY);
    }

    // Copy horiz bits
    if (visible || prerender) && dot == 257 {
        ops = ops.push(PpuOperation::CopyHorizV);
    }

    // Copy vertical bits
    if prerender && (dot >= 280 && dot <= 304) {
        ops = ops.push(PpuOperation::CopyVertV);
    }

    // Clear secondary OAM (dots 1-64)
    if visible && dot >= 1 && dot <= 64 {
        ops = ops.push(PpuOperation::ClearSecondaryOam);
    }

    // Reset sprite evaluation (dot 65)
    if visible && dot == 65 {
        ops = ops.push(PpuOperation::ResetSpriteEvaluation);
    }

    // Sprite eval (ODD dots 65-256)
    if visible && (dot >= 65 && dot <= 256) && dot % 2 == 1 {
        ops = ops.push(PpuOperation::EvaluateSprites);
    }

    // FIXME: This doesn't match the NESDev PPU.svg chart...
    // Sprite fetches (dots 257-320)
    if (visible || prerender) && (dot >= 257 && dot <= 320) && (dot - 257).is_multiple_of(8) {
        ops = ops.push(PpuOperation::FillSpriteRegister);
    }

    // VBlank flag toggle
    if dot == 1 {
        if scanline == 241 {
            ops = ops.push(PpuOperation::SetVBlank);
        }
        if prerender {
            ops = ops.push(PpuOperation::ClearVBlank);
        }
    }

    ops
}
