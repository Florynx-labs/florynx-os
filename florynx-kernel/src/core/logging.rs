// =============================================================================
// Florynx Kernel — Logging System
// =============================================================================
// Provides kernel-level logging macros with severity levels.
// Output goes to serial for debug builds.
// =============================================================================

/// Log level enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Log a message with a given level.
#[macro_export]
macro_rules! klog {
    ($level:expr, $($arg:tt)*) => {{
        $crate::serial_println!("[{}] {}", match $level {
            $crate::core_kernel::logging::LogLevel::Trace => "TRACE",
            $crate::core_kernel::logging::LogLevel::Debug => "DEBUG",
            $crate::core_kernel::logging::LogLevel::Info  => "INFO ",
            $crate::core_kernel::logging::LogLevel::Warn  => "WARN ",
            $crate::core_kernel::logging::LogLevel::Error => "ERROR",
        }, format_args!($($arg)*));
    }};
}

/// Log at TRACE level.
#[macro_export]
macro_rules! klog_trace {
    ($($arg:tt)*) => ($crate::klog!($crate::core_kernel::logging::LogLevel::Trace, $($arg)*));
}

/// Log at DEBUG level.
#[macro_export]
macro_rules! klog_debug {
    ($($arg:tt)*) => ($crate::klog!($crate::core_kernel::logging::LogLevel::Debug, $($arg)*));
}

/// Log at INFO level.
#[macro_export]
macro_rules! klog_info {
    ($($arg:tt)*) => ($crate::klog!($crate::core_kernel::logging::LogLevel::Info, $($arg)*));
}

/// Log at WARN level.
#[macro_export]
macro_rules! klog_warn {
    ($($arg:tt)*) => ($crate::klog!($crate::core_kernel::logging::LogLevel::Warn, $($arg)*));
}

/// Log at ERROR level.
#[macro_export]
macro_rules! klog_error {
    ($($arg:tt)*) => ($crate::klog!($crate::core_kernel::logging::LogLevel::Error, $($arg)*));
}
