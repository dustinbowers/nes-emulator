pub const DOTS: usize = 341;
pub const SCAN_LINES: usize = 262;

#[cfg(not(feature = "runtime-ppu-schedule"))]
static PPU_SCHEDULE: [[DotOperations; DOTS]; SCAN_LINES] = build_schedule();

#[cfg(feature = "runtime-ppu-schedule")]
use std::sync::OnceLock;
#[cfg(feature = "runtime-ppu-schedule")]
pub static PPU_SCHEDULE: OnceLock<[[DotOperations; DOTS]; SCAN_LINES]> = OnceLock::new();

#[inline(always)]
pub fn get_ppu_schedule() -> &'static [[DotOperations; DOTS]; SCAN_LINES] {
    #[cfg(feature = "runtime-ppu-schedule")]
    {
        PPU_SCHEDULE.get_or_init(build_schedule)
    }
    #[cfg(not(feature = "runtime-ppu-schedule"))]
    {
        &PPU_SCHEDULE
    }
}

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
#[derive(Debug, Copy, Clone, PartialEq)]
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
    ClearVBlank2,

    None,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct DotOperations {
    pub len: u8,
    pub ops: [PpuOperation; 7],
}

impl DotOperations {
    pub const fn new() -> Self {
        Self {
            len: 0,
            ops: [PpuOperation::None; 7],
        }
    }

    pub const fn push(mut self, op: PpuOperation) -> Self {
        debug_assert!(self.len < 7);
        self.ops[self.len as usize] = op;
        self.len += 1;
        self
    }
}

#[inline(always)]
pub fn ppu_schedule() -> &'static [[DotOperations; DOTS]; SCAN_LINES] {
    get_ppu_schedule()
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
    let rendering = visible || prerender;

    // Background fetch windows
    let bg_fetch_window = rendering && ((dot >= 1 && dot <= 256) || (dot >= 321 && dot <= 336));

    // Sprite fetch window
    let sprite_fetch_window = rendering && (dot >= 257 && dot <= 320);

    // Render a pixel (dots 1-256) before shift for this dot
    if visible && dot >= 1 && dot <= 256 {
        ops = ops.push(PpuOperation::RenderPixel);
    }

    if rendering && ((dot >= 1 && dot <= 256) || (dot >= 321 && dot <= 336)) {
        ops = ops.push(PpuOperation::ShiftRegisters);
    }

    // Background fetch pipeline
    if bg_fetch_window {
        match dot % 8 {
            1 => ops = ops.push(PpuOperation::FetchNameTable),
            3 => ops = ops.push(PpuOperation::FetchAttribute),
            5 => ops = ops.push(PpuOperation::FetchTileLow),
            7 => ops = ops.push(PpuOperation::FetchTileHigh),
            _ => {}
        }
    }

    // Load background shift registers
    if bg_fetch_window && dot % 8 == 0 {
        ops = ops.push(PpuOperation::LoadBackgroundRegisters);
    }

    // Increment coarse X at end of tile (visible + prefetch windows)
    if rendering && dot % 8 == 0 && ((dot >= 8 && dot <= 256) || (dot >= 328 && dot <= 336)) {
        ops = ops.push(PpuOperation::IncCoarseX);
    }

    // Fine Y increments
    if (visible || prerender) && dot == 256 {
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
    // Sprite fetches
    if sprite_fetch_window && dot % 2 == 0 {
        ops = ops.push(PpuOperation::FillSpriteRegister);
    }

    // VBlank flag toggle
    if scanline == 241 {
        if dot == 1 {
            ops = ops.push(PpuOperation::SetVBlank);
        }
    }

    if prerender {
        // Clear VBL and sprite flags at the start of pre-render
        if dot == 1 {
            ops = ops.push(PpuOperation::ClearVBlank);
            ops = ops.push(PpuOperation::ClearVBlank2);
        }
    }

    ops
}

#[cfg(test)]
mod test {
    use super::*;

    fn has_op(ops: &DotOperations, op: PpuOperation) -> bool {
        ops.ops[..ops.len as usize].contains(&op)
    }

    fn assert_has(scanline: usize, dot: usize, op: PpuOperation) {
        let ops = &get_ppu_schedule()[scanline][dot];
        assert!(
            has_op(ops, op),
            "Expected {:?} at scanline {}, dot {} but got {:?}",
            op,
            scanline,
            dot,
            &ops.ops[..ops.len as usize]
        );
    }

    fn assert_not(scanline: usize, dot: usize, op: PpuOperation) {
        let ops = &get_ppu_schedule()[scanline][dot];
        assert!(
            !has_op(ops, op),
            "Did not expect {:?} at scanline {}, dot {} but got {:?}",
            op,
            scanline,
            dot,
            &ops.ops[..ops.len as usize]
        );
    }

    #[test]
    fn test_background_fetch_pipeline() {
        let scanline = 0;

        for dot in 1..=256 {
            let ops = &get_ppu_schedule()[scanline][dot];

            // Always shift during fetch cycles
            if dot >= 1 {
                assert!(has_op(ops, PpuOperation::ShiftRegisters));
            } else {
                assert_not(scanline, dot, PpuOperation::ShiftRegisters);
            }

            match dot % 8 {
                0 if dot >= 8 => {
                    assert!(has_op(ops, PpuOperation::LoadBackgroundRegisters));
                    assert!(has_op(ops, PpuOperation::IncCoarseX));
                }
                1 => assert!(has_op(ops, PpuOperation::FetchNameTable)),
                3 => assert!(has_op(ops, PpuOperation::FetchAttribute)),
                5 => assert!(has_op(ops, PpuOperation::FetchTileLow)),
                7 => assert!(has_op(ops, PpuOperation::FetchTileHigh)),
                _ => {}
            }
        }
    }

    #[test]
    fn test_render_only_on_visible_dots() {
        for scanline in 0..240 {
            for dot in 1..=256 {
                assert_has(scanline, dot, PpuOperation::RenderPixel);
            }
            for dot in [0, 257, 320, 340] {
                assert_not(scanline, dot, PpuOperation::RenderPixel);
            }
        }

        // No rendering in vblank or prerender
        for scanline in [241, 260, 261] {
            for dot in 0..341 {
                assert_not(scanline, dot, PpuOperation::RenderPixel);
            }
        }
    }

    #[test]
    fn test_sprite_evaluation_timing() {
        let scanline = 10;

        for dot in 65..=256 {
            if dot % 2 == 1 {
                assert_has(scanline, dot, PpuOperation::EvaluateSprites);
            } else {
                assert_not(scanline, dot, PpuOperation::EvaluateSprites);
            }
        }
        for dot in 0..64 {
            assert_not(scanline, dot, PpuOperation::EvaluateSprites);
        }
    }

    #[test]
    fn test_secondary_oam_clear() {
        let scanline = 20;

        for dot in 1..=64 {
            assert_has(scanline, dot, PpuOperation::ClearSecondaryOam);
        }
        for dot in 65..341 {
            assert_not(scanline, dot, PpuOperation::ClearSecondaryOam);
        }
    }

    #[test]
    fn trace_inc_y_timing() {
        // Visible
        let ops = get_ppu_schedule()[0][256];
        assert!(ops.ops[..ops.len as usize].contains(&PpuOperation::IncFineY));

        // Pre-render
        let ops = get_ppu_schedule()[261][256];
        assert!(ops.ops[..ops.len as usize].contains(&PpuOperation::IncFineY));
    }

    #[test]
    fn trace_vertical_reload_window() {
        for dot in 280..=304 {
            let ops = &get_ppu_schedule()[261][dot];
            assert!(
                ops.ops[..ops.len as usize].contains(&PpuOperation::CopyVertV),
                "Missing CopyVertV at prerender dot {}",
                dot
            );
        }
    }

    #[test]
    fn test_sprite_eval_reset() {
        let scanline = 50;

        assert_has(scanline, 65, PpuOperation::ResetSpriteEvaluation);
        assert_not(scanline, 64, PpuOperation::ResetSpriteEvaluation);
        assert_not(scanline, 66, PpuOperation::ResetSpriteEvaluation);
    }

    #[test]
    fn test_scroll_timing() {
        for scanline in 0..240 {
            assert_has(scanline, 256, PpuOperation::IncFineY);
            assert_has(scanline, 257, PpuOperation::CopyHorizV);
        }

        // Only prerender has CopyVert
        for dot in 280..=304 {
            assert_has(261, dot, PpuOperation::CopyVertV);
        }
    }

    #[test]
    fn test_vblank_timing() {
        // VBlank set
        assert_has(241, 1, PpuOperation::SetVBlank);

        // VBlank clear (pre-render)
        assert_has(261, 0, PpuOperation::ClearVBlank);

        // Should not appear elsewhere
        for scanline in 0..262 {
            for dot in 0..341 {
                if !(scanline == 241 && dot == 1) {
                    assert_not(scanline, dot, PpuOperation::SetVBlank);
                }
                if !(scanline == 261 && dot == 0) {
                    assert_not(scanline, dot, PpuOperation::ClearVBlank);
                }
            }
        }
    }

    #[test]
    fn test_no_fetches_during_vblank() {
        for scanline in 241..261 {
            for dot in 0..341 {
                let ops = &get_ppu_schedule()[scanline][dot];
                assert!(!has_op(ops, PpuOperation::FetchNameTable));
                assert!(!has_op(ops, PpuOperation::FetchAttribute));
                assert!(!has_op(ops, PpuOperation::FetchTileLow));
                assert!(!has_op(ops, PpuOperation::FetchTileHigh));
            }
        }
    }

    #[test]
    fn test_sprite_fetch_window() {
        let scanline = 0;

        for dot in 257..=320 {
            if (dot - 257) % 8 == 0 {
                assert_has(scanline, dot, PpuOperation::FillSpriteRegister);
            } else {
                assert_not(scanline, dot, PpuOperation::FillSpriteRegister);
            }
        }
    }
}
