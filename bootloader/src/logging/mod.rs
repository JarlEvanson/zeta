//! Logging for the bootloader.
//!
//! Controls filtering and output of log messages.
//!
//! Before parsing the configuration file, logging is controlled by environment
//! variables in the `pre_config` module.

use core::{fmt::Write, sync::atomic::Ordering};

use log::{Level, LevelFilter};
use sync::Mutex;
use uefi::{
    boot::{get_handle_for_protocol, open_protocol_exclusive, ScopedProtocol},
    proto::console::serial::Serial,
    Status,
};

use self::filter::AtomicLevelFilter;

mod filter;
mod preconfig;

/// The filter for all logs.
///
/// When initialized, set to the value of `PRE_CONFIG_GLOBAL`.
static GLOBAL_FILTER: AtomicLevelFilter = AtomicLevelFilter::new(LevelFilter::Off);

/// The state of the serial output for logging.
static mut SERIAL_STATE: Mutex<SerialState> = Mutex::new(SerialState::Uninitialized);
/// The filter for all logs that go through the serial output.
///
/// When initialized, set to the value of `PRE_CONFIG_SERIAL`.
static SERIAL_FILTER: AtomicLevelFilter = AtomicLevelFilter::new(LevelFilter::Off);

/// The private representation of the logger.
struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= GLOBAL_FILTER.load(Ordering::Relaxed)
    }

    fn log(&self, record: &log::Record) {
        if record.level() > GLOBAL_FILTER.load(Ordering::Relaxed) {
            return;
        }

        log_serial(record);
    }

    fn flush(&self) {}
}

/// Passes a log through to the serial output provided [`SERIAL_FILTER`] doesn't filter it out.
fn log_serial(record: &log::Record) {
    if record.level() > SERIAL_FILTER.load(Ordering::Relaxed) {
        return;
    }

    // SAFETY:
    // UEFI is single threaded, so `SerialState` is safe to access.
    let mut guard = unsafe { SERIAL_STATE.lock() };

    match &mut *guard {
        SerialState::Uninitialized => {}
        SerialState::Proto(proto) => log(&mut **proto, record),
    }
}

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

/// Initializes the logging framework, setting the filters to the corresponding [`LevelFilter`]
/// specified by the environment variables.
///
/// The global filter takes on the value specified by `PRE_CONFIG_GLOBAL`.
/// The serial filter takes on the value specified by `PRE_CONFIG_SERIAL`.
///
/// # Panics
/// Panics if this function has already been called and returned successfully.
///
/// # Errors
/// If an error occurs while obtaining a serial logger, an error is returned relating to that.
pub fn initialize() -> Result<(), SetupLoggingError> {
    match acquire_serial() {
        // SAFETY:
        // UEFI is single threaded, so `SerialState` is safe to access.
        Ok(serial) => unsafe {
            *SERIAL_STATE.lock() = SerialState::Proto(serial);
        },
        Err(err) => return Err(err),
    }
    SERIAL_FILTER.store(preconfig::PRECONFIG_SERIAL, Ordering::Relaxed);

    GLOBAL_FILTER.store(preconfig::PRECONFIG_GLOBAL, Ordering::Relaxed);

    log::set_logger(&Logger).expect("logger can only be set once");
    log::set_max_level(LevelFilter::Trace);

    Ok(())
}

/// Attempts to acquire a serial output.
///
/// # Errors
/// - [`OutOfResources`][oof]
///     - Returned when there is not enough resources to acquire a handle for a protocol.
/// - [`NotFound`][nf]
///     - Returned when no handle is found supporting the [`Serial`] protocol.
/// - [`AccessDenied`][ac]
///     - Returned when the attempt to open the [`Serial`] protocl failed.
/// - [`General`][g]
///     - Returned for all other errors.
///
/// [oof]: SetupLoggingError::OutOfResources
/// [nf]: SetupLoggingError::NotFound
/// [ac]: SetupLoggingError::AccessDenied
/// [g]: SetupLoggingError::General
fn acquire_serial() -> Result<ScopedProtocol<'static, Serial>, SetupLoggingError> {
    let handle = match get_handle_for_protocol::<Serial>() {
        Ok(handle) => handle,
        Err(err) => match err.status() {
            Status::NOT_FOUND => return Err(SetupLoggingError::NotFound),
            Status::OUT_OF_RESOURCES => return Err(SetupLoggingError::OutOfResources),
            _ => return Err(SetupLoggingError::General),
        },
    };

    match open_protocol_exclusive::<Serial>(handle) {
        Ok(mut serial) => {
            serial.reset().map_err(|_| SetupLoggingError::General)?;
            Ok(serial)
        }
        Err(err) => match err.status() {
            Status::ACCESS_DENIED => Err(SetupLoggingError::AccessDenied),
            _ => Err(SetupLoggingError::General),
        },
    }
}

/// Various errors returned when setting up a logger.
pub enum SetupLoggingError {
    /// There wasn't enough resources to find a valid handle supporting the requested protocol.
    OutOfResources,
    /// No handles support the requested protocol.
    NotFound,
    /// Access to the requested protocol was denied.
    AccessDenied,
    /// An unsupported error was returned.
    General,
}

/// The state of the serial logging facility.
enum SerialState {
    /// Uninitialized.
    ///
    /// Can occur both before setting up logging and after boot services are exited.
    Uninitialized,
    /// Protocol setup.
    Proto(ScopedProtocol<'static, Serial>),
}
