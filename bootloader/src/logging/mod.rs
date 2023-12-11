//! Logging for the bootloader.
//!
//! Controls filtering and output of log messages.
//!
//! Before parsing the configuration file, logging is controlled by environment
//! variables in the `pre_config` module.

use log::LevelFilter;

#[cfg(any(feature = "serial_logging", feature = "framebuffer_logging"))]
use core::{fmt::Write, sync::atomic::Ordering};
#[cfg(any(feature = "serial_logging", feature = "framebuffer_logging"))]
use filter::AtomicLevelFilter;
#[cfg(any(feature = "serial_logging", feature = "framebuffer_logging"))]
use log::Level;

#[cfg(any(feature = "serial_logging", feature = "framebuffer_logging"))]
mod filter;
mod preconfig;
mod state;

pub use state::*;

/// The filter for all logs that go through the serial output.
///
/// When initialized, set to the value of `PRE_CONFIG_SERIAL`.
#[cfg(feature = "serial_logging")]
static SERIAL_FILTER: AtomicLevelFilter = AtomicLevelFilter::new(LevelFilter::Off);

/// The filter for all logs that go through the serial output.
///
/// When initialized, set to the value of `PRE_CONFIG_FRAMEBUFFER`.
#[cfg(feature = "framebuffer_logging")]
static FRAMEBUFFER_FILTER: AtomicLevelFilter = AtomicLevelFilter::new(LevelFilter::Off);

/// Atomically updates the global logging to `filter`.
pub fn set_global_filter(filter: LevelFilter) {
    log::set_max_level(filter);
}

/// Atomically updates [`SERIAL_FILTER`] to `filter`.
#[cfg_attr(not(feature = "serial_logging"), allow(unused))]
pub fn set_serial_filter(filter: LevelFilter) {
    #[cfg(feature = "serial_logging")]
    SERIAL_FILTER.store(filter, Ordering::Relaxed);
}

/// Atomically updates [`FRAMEBUFFER_FILTER`] to `filter`.
#[cfg_attr(not(feature = "framebuffer_logging"), allow(unused))]
pub fn set_framebuffer_filter(filter: LevelFilter) {
    #[cfg(feature = "framebuffer_logging")]
    FRAMEBUFFER_FILTER.store(filter, Ordering::Relaxed);
}

/// The private representation of the logger.
struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    #[cfg_attr(
        not(all(feature = "serial_logging", feature = "framebuffer_logging")),
        allow(unused)
    )]
    fn log(&self, record: &log::Record) {
        #[cfg(feature = "serial_logging")]
        log_serial(record);

        #[cfg(feature = "framebuffer_logging")]
        log_framebuffer(record);
    }

    fn flush(&self) {}
}

#[cfg(feature = "serial_logging")]
/// Passes a log through to the serial output provided [`SERIAL_FILTER`] doesn't filter it out.
fn log_serial(record: &log::Record) {
    if record.level() > SERIAL_FILTER.load(Ordering::Relaxed) {
        return;
    }

    let mut guard = serial_logger();

    match &mut *guard {
        SerialState::Uninitialized => {}
        SerialState::Proto(proto) => log(&mut **proto, record),
    }
}

#[cfg(feature = "framebuffer_logging")]
/// Passes a log through to the framebuffer provided [`FRAMEBUFFER_FILTER`] doesn't filter it out.
fn log_framebuffer(record: &log::Record) {
    if record.level() > FRAMEBUFFER_FILTER.load(Ordering::Relaxed) {
        return;
    }

    let _ = core::hint::black_box(8);
}

#[cfg(any(feature = "serial_logging", feature = "framebuffer_logging"))]
/// Generic implementation of the logging output.
#[track_caller]
fn log<W>(mut w: &mut W, record: &log::Record)
where
    for<'a> &'a mut W: Write,
{
    if record.target() == "panic" {
        // Would cause a double panic.
        let _ = writeln!(w, "[PANIC]: {}", record.args());
        return;
    }
    match record.level() {
        Level::Debug | Level::Trace
            if record.module_path().is_some() && record.line().is_some() =>
        {
            writeln!(
                w,
                "[{}: {}] at {}:{} => {}",
                record.level(),
                record.target(),
                record.module_path().unwrap(),
                record.line().unwrap(),
                record.args()
            )
            .unwrap();
        }
        _ => writeln!(
            w,
            "[{}: {}] {}",
            record.level(),
            record.target(),
            record.args()
        )
        .unwrap(),
    }
}
