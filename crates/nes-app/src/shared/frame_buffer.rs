use std::cell::UnsafeCell;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const W: usize = 256;
const H: usize = 240;
type Frame = [u8; W * H];

pub type SharedFrameHandle = Arc<SharedFrame>;
pub struct SharedFrame {
    active: AtomicUsize,
    buffers: [UnsafeCell<Frame>; 2],
}

// SAFETY: Single-publisher-single-consumer must be followed
// - UI thread only reads the buffer selected by `active`
// - emulation thread only writes to the non-active buffer, then publishes
unsafe impl Sync for SharedFrame {}

impl SharedFrame {
    pub fn new() -> Self {
        Self {
            active: AtomicUsize::new(0),
            buffers: [
                UnsafeCell::new([0u8; W * H]),
                UnsafeCell::new([0u8; W * H]),
            ],
        }
    }

    #[inline]
    pub fn read(&self) -> &Frame {
        let index = self.active.load(Ordering::Acquire);

        // SAFETY: read() only reads the active buffer
        unsafe { &*self.buffers[index].get() }
    }

    #[inline]
    pub fn write(&self, frame: &Frame) {
        let index = self.active.load(Ordering::Relaxed);
        let other = index ^ 1;

        // SAFETY: write() only writes to non-active buffer
        unsafe { (*self.buffers[other].get()).copy_from_slice(frame) };

        self.active.store(other, Ordering::Release);
    }
}
