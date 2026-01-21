use std::sync::Mutex;
use once_cell::sync::Lazy;
use crate::nes::tracer::Tracer;

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
macro_rules! trace_dump {
    () => {
        #[cfg(feature = "tracing")]
        {
            $crate::nes::tracer::TRACER.lock().unwrap().print();
        }
    };
}

#[macro_export]
macro_rules! trace_obj {
    ($obj:expr) => {
        #[cfg(feature = "tracing")]
        {
            $crate::nes::tracer::TRACER.lock().unwrap().log($obj);
        }
    };
}

#[macro_export]
macro_rules! trace_ppu_event {
    ($($arg:tt)*) => {
        #[cfg(feature = "tracing")]
        {
            trace!("[PPU EVENT] {}", format!($($arg)*));
        }
    };
}

#[cfg(feature = "tracing")]
pub static TRACER: Lazy<Mutex<Tracer>> = Lazy::new(|| Mutex::new(Tracer::new(1_000_000)));

