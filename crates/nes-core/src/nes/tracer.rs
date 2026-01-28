use once_cell::sync::Lazy;
use std::collections::VecDeque;
use std::sync::Mutex;

mod macros;
pub mod traceable;
pub use traceable::Traceable;

pub static TRACER: Lazy<Mutex<Tracer>> = Lazy::new(|| Mutex::new(Tracer::new(5_000_000)));

/// Global tracer
pub struct Tracer {
    history: VecDeque<String>,
    capacity: usize,
}

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
            self.write(trace.to_string());
        }
    }

    pub fn clear(&mut self) {
        self.history.clear();
    }
}
