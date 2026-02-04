
pub trait AppEventSource {
    // TODO
    fn poll_event(&mut self) -> Option<AppEvent>;
}

#[derive(Debug)]
pub enum AppEvent {
    Start,
    LoadRom(Vec<u8>),
    Run,
    Pause,
    Reset,
}
