pub struct SharedFrame {
    pub pixels: [u8; 256 * 240],
    pub dirty: bool,
}

impl SharedFrame {
    pub fn new() -> Self {
        Self {
            pixels: [0u8; 256 * 240],
            dirty: false,
        }
    }
}
