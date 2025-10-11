pub trait Traceable {
    /// Return a short type label (e.g., "CPU", "PPU")
    fn trace_name(&self) -> &'static str;

    /// State string (whatever details matter for this component)
    fn trace_state(&self) -> Option<String>;

    // fn trace(&self) {
    //     tracing::trace!("{}: {}", self.tracer_name(), self.trace_state());
    // }
    fn trace(&self) -> Option<String> {
        if let Some(state) = self.trace_state() {
            Some(format!("{} {}", self.trace_name(), state))
        } else {
            None
        }
    }
}
