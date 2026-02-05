#[cfg(feature = "tracing")]
#[inline(always)]
pub(crate) fn __trace_write(args: std::fmt::Arguments) {
    let mut tracer = crate::nes::tracer::TRACER.lock().unwrap();
    tracer.write(args.to_string());
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        #[cfg(feature = "tracing")]
        {
            $crate::nes::tracer::macros::__trace_write(format_args!($($arg)*));
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
            $crate::nes::tracer::macros::__trace_write(
                format_args!("[PPU EVENT] {}", format_args!($($arg)*))
            );
        }
    };
}

#[macro_export]
macro_rules! trace_cpu_event {
    ($($arg:tt)*) => {
        #[cfg(feature = "tracing")]
        {
            $crate::nes::tracer::macros::__trace_write(
                format_args!("[CPU EVENT] {}", format_args!($($arg)*))
            );
        }
    };
}
