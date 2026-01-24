#[derive(Debug)]
pub enum AppEvent {
    LoadRom(Vec<u8>),
    Reset,
    Pause,
}

pub trait AppEventSource {
    fn poll_event(&mut self) -> Option<AppEvent>;
}
