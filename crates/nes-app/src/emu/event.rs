use std::borrow::Cow;

/// EmuEvents are sent Audio -> UI
pub enum EmuEvent {
    Log(Cow<'static, str>),
}
