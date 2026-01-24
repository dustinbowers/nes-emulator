#[derive(Debug)]
pub enum AppEvent {
    RequestLoadRom(Vec<u8>),
    RequestReset,
    RequestPause,
}

pub trait AppEventSource {
    fn poll_event(&mut self) -> Option<AppEvent>;
}
