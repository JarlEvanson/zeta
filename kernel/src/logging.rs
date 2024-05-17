//! Zeta kernel logging.

use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use crate::spinlock::{RawSpinLock, SpinLock};

/// The filter on logging that is currently active.
static LEVEL_FILTER: AtomicU8 = AtomicU8::new(LogFilter::Off as u8);

/// Sets the global [`LogFilter`].
pub fn set_filter(level: LogFilter) {
    LEVEL_FILTER.store(level as u8, Ordering::Relaxed);
}

/// Returns the currently active global [`LogFilter`].
pub fn get_filter() -> LogFilter {
    match LEVEL_FILTER.load(Ordering::Relaxed) {
        TRACE => LogFilter::Trace,
        DEBUG => LogFilter::Debug,
        INFO => LogFilter::Info,
        WARN => LogFilter::Warn,
        ERROR => LogFilter::Error,
        FATAL => LogFilter::Fatal,
        OFF => LogFilter::Off,
        _ => unreachable!(),
    }
}

/// The active logger.
static mut LOGGER: &mut dyn Logger = &mut NullLogger;

/// The lock on the logger.
static LOCK: RawSpinLock = RawSpinLock::new();

/// Writes `args` to the current global [`Logger`] at `level` severity.
pub fn log_fmt(level: LogLevel, args: core::fmt::Arguments) {
    LOCK.lock();

    // SAFETY:
    // Mutable access is properly protected by [`LOCK`].
    unsafe { LOGGER.log(level, args) };

    LOCK.unlock();
}

/// Sets the current global [`Logger`].
pub fn set_logger(logger: &'static mut dyn Logger) {
    LOCK.lock();

    // SAFETY:
    // Mutable access is properly protected by [`LOCK`].
    unsafe {
        LOGGER = logger;
    }

    LOCK.unlock();
}

/// The generic logging macro.
///
/// This generically logs with the specified `level` and `format!` based arguments.
#[macro_export]
macro_rules! log {
    ($level:expr, $($arg:tt)*) => {
        {
            $crate::logging::log_fmt($level, core::format_args!("{}\n", core::format_args!($($arg)*)));
        }
    };
}

/// Logs the provided `format!` based arguments at [`LogLevel::Trace`].
#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => {
        {
            $crate::log!($crate::logging::LogLevel::Trace, $($arg)*);
        }
    };
}

/// Logs the provided `format!` based arguments at [`LogLevel::Debug`].
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        {
            $crate::log!($crate::logging::LogLevel::Debug, $($arg)*);
        }
    };
}

/// Logs the provided `format!` based arguments at [`LogLevel::Info`].
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        {
            $crate::log!($crate::logging::LogLevel::Info, $($arg)*);
        }
    };
}

/// Logs the provided `format!` based arguments at [`LogLevel::Warn`].
#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        {
            $crate::log!($crate::logging::LogLevel::Warn, $($arg)*);
        }
    };
}

/// Logs the provided `format!` based arguments at [`LogLevel::Error`].
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        {
            $crate::log!($crate::logging::LogLevel::Error, $($arg)*);
        }
    };
}

/// Logs the provided `format!` based arguments at [`LogLevel::Fatal`].
#[macro_export]
macro_rules! log_fatal {
    ($($arg:tt)*) => {
        {
            $crate::log!($crate::logging::LogLevel::Fatal, $($arg)*);
        }
    };
}

/// Implements a logger.
pub trait Logger {
    /// Immediately outputs `args` at `level`.
    fn log(&mut self, level: LogLevel, args: core::fmt::Arguments);
}

/// A logger that does outputs nothing.
struct NullLogger;

impl Logger for NullLogger {
    fn log(&mut self, _level: LogLevel, _args: core::fmt::Arguments) {}
}

/// The integer constant of [`LogFilter::Trace`].
const TRACE: u8 = 0;
/// The integer constant corresponding to [`LogFilter::Debug`].
const DEBUG: u8 = 1;
/// The integer constant corresponding to [`LogFilter::Info`].
const INFO: u8 = 2;
/// The integer constant corresponding to [`LogFilter::Warn`].
const WARN: u8 = 3;
/// The integer constant corresponding to [`LogFilter::Error`].
const ERROR: u8 = 4;
/// The integer constant corresponding to [`LogFilter::Fatal`].
const FATAL: u8 = 5;
/// The integer constant corresponding to [`LogFilter::Off`].
const OFF: u8 = 6;

/// Logging levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum LogLevel {
    /// The most fine-grained of all the [`LogLevel`]'s, this is used to trace the execution throughout
    /// the program.
    Trace = LogFilter::Trace as u8,
    /// Less granular compared to [`LogLevel::Trace`], this is used for information that may be used to
    /// diagnose issues, troubleshooting, or when running an application in a test environment to make sure
    /// everything is running correctly.
    Debug = LogFilter::Debug as u8,
    /// The standard log level indicating something happended. This should be purely informative and not
    /// checking these logs shouldn't result in missing any important information.
    Info = LogFilter::Info as u8,
    /// The log level that indicates that something unexpected happended in the application, a problem, or
    /// a situation that might cause problems in the future. Should be used in situations that are unexpected,
    /// but the code can continue the work.
    Warn = LogFilter::Warn as u8,
    /// The [`LogLevel`] that should be used when the program encounters an issue that prevents one or more
    /// functionalities from working.
    Error = LogFilter::Error as u8,
    /// The [`LogLevel`] that reports that the application encountered an issue in which one of the most important
    /// systems is no longer working.
    Fatal = LogFilter::Fatal as u8,
}

/// Controls which logs are outputted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(u8)]
pub enum LogFilter {
    /// Allows all log levels of equal or less severity than [`LogLevel::Trace`].
    #[default]
    Trace = TRACE,
    /// Allows all log levels of equal or less severity than [`LogLevel::Debug`].
    Debug = DEBUG,
    /// Allows all log levels of equal or less severity than [`LogLevel::Info`].
    Info = INFO,
    /// Allows all log levels of equal or less severity than [`LogLevel::Warn`].
    Warn = WARN,
    /// Allows only logs of [`LogLevel::Error`] or [`LogLevel::Fatal`].
    Error = ERROR,
    /// Allows only logs of [`LogLevel::Fatal`].
    Fatal = FATAL,
    /// Disables all logs.
    Off = OFF,
}
