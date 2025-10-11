pub mod traceable;

use crate::nes::tracer::traceable::Traceable;
use once_cell::sync::Lazy;
use std::collections::VecDeque;
use std::sync::Mutex;

#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        #[cfg(feature = "tracing")]
        {
            $crate::nes::tracer::TRACER.lock().unwrap().write(format!($($arg)*));
        }
    };
}

#[macro_export]
macro_rules! trace_obj {
    ($obj:expr) => {
        #[cfg(feature = "tracing")]
        {
            $crate::nes::tracer::TRACER.lock().unwrap().log(&$obj);
        }
    };
}

#[cfg(feature = "tracing")]
pub static TRACER: Lazy<Mutex<Tracer>> = Lazy::new(|| Mutex::new(Tracer::new(5_000_000)));

/// Global tracer (enabled only if `tracing` feature is active)
#[cfg(feature = "tracing")]
pub struct Tracer {
    history: VecDeque<String>,
    capacity: usize,
}

#[cfg(feature = "tracing")]
impl Tracer {
    pub fn new(capacity: usize) -> Self {
        Self {
            history: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn write(&mut self, msg: String) {
        if self.history.len() == self.capacity {
            self.history.pop_front();
        }
        self.history.push_back(msg);
    }

    pub fn print(&self) {
        for (i, line) in self.history.iter().enumerate() {
            println!("{:04}: {}", i, line);
        }
    }

    pub fn log<T: Traceable>(&mut self, thing: &T) {
        let trace = thing.trace();
        if let Some(trace) = trace {
            self.write(format!("{}", trace));
        }
    }

    pub fn clear(&mut self) {
        self.history.clear();
    }
}

// No-op stub for when tracing is disabled
#[cfg(not(feature = "tracing"))]
pub struct Tracer;

#[cfg(not(feature = "tracing"))]
impl Tracer {
    pub fn new(_: usize) -> Self {
        Tracer
    }
    pub fn write(&mut self, _: String) {}
    pub fn print(&self) {}
    pub fn clear(&mut self) {}
}
