#[derive(Debug)]
pub enum EmulatorMessage {
    // JS to WASM
    LoadRom(Vec<u8>),
    Reset,
    Pause,

    // WASM to JS
    Ready,
    Error(String),
}

pub struct Envelope {
    pub version: u32,
    pub message: EmulatorMessage,
}
