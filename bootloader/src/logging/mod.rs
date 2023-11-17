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

/// The filter for all logs.
///
/// When initialized, set to the value of `PRE_CONFIG_GLOBAL`.
static GLOBAL_FILTER: AtomicLevelFilter = AtomicLevelFilter::new(LevelFilter::Off);

/// The state of the serial output for logging.
static SERIAL_STATE: Mutex<SerialState> = Mutex::new(SerialState::Uninitialized);
/// The filter for all logs that go through the serial output.
///
/// When initialized, set to the value of `PRE_CONFIG_SERIAL`.
static SERIAL_FILTER: AtomicLevelFilter = AtomicLevelFilter::new(LevelFilter::Off);

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
        Ok(serial) => *SERIAL_STATE.lock() = SerialState::Proto(serial),
        Err(err) => return Err(err),
    }
    SERIAL_FILTER.store(pre_config::PRE_CONFIG_SERIAL, Ordering::Relaxed);

    GLOBAL_FILTER.store(pre_config::PRE_CONFIG_GLOBAL, Ordering::Relaxed);

    log::set_logger(&Logger).expect("logger can only be set once");
    log::set_max_level(LevelFilter::Trace);

    Ok(())
}

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= GLOBAL_FILTER.load(Ordering::Relaxed)
    }

    fn log(&self, record: &log::Record) {
        if record.level() <= GLOBAL_FILTER.load(Ordering::Relaxed)
            && record.level() <= SERIAL_FILTER.load(Ordering::Relaxed)
        {
            log_serial(record);
        }
    }

    fn flush(&self) {}
}

fn log_serial(record: &log::Record) {
    let mut guard = SERIAL_STATE.lock();

    match &mut *guard {
        SerialState::Uninitialized => {}
        SerialState::Proto(proto) => log(&mut **proto, record),
    }
}

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

pub enum SetupLoggingError {
    OutOfResources,
    NotFound,
    AccessDenied,
    General,
}

enum SerialState {
    Uninitialized,
    Proto(ScopedProtocol<'static, Serial>),
}

unsafe impl Send for SerialState {}

mod pre_config {
    use log::LevelFilter;

    pub const PRE_CONFIG_GLOBAL: LevelFilter =
        match level_filter_from_str(match core::option_env!("PRE_CONFIG_GLOBAL") {
            Some(env) => env,
            None => "error",
        }) {
            Some(filter) => filter,
            None => panic!(
                "`BOOT_GLOBAL` must be either `off`, `error`, `warn`, `info`, `debug`, or `trace`"
            ),
        };
    pub const PRE_CONFIG_SERIAL: LevelFilter =
        match level_filter_from_str(match core::option_env!("PRE_CONFIG_SERIAL") {
            Some(env) => env,
            None => "error",
        }) {
            Some(filter) => filter,
            None => panic!(
                "`BOOT_GLOBAL` must be either `off`, `error`, `warn`, `info`, `debug`, or `trace`"
            ),
        };
    pub const PRE_CONFIG_FRAMEBUFFER: LevelFilter =
        match level_filter_from_str(match core::option_env!("PRE_CONFIG_FRAMEBUFFER") {
            Some(env) => env,
            None => "error",
        }) {
            Some(filter) => filter,
            None => panic!(
                "`BOOT_GLOBAL` must be either `off`, `error`, `warn`, `info`, `debug`, or `trace`"
            ),
        };

    const fn level_filter_from_str(filter: &str) -> Option<LevelFilter> {
        const fn eq_ignore_ascii_case(lhs: &str, rhs: &str) -> bool {
            if lhs.len() != rhs.len() {
                return false;
            }

            let mut byte_pos = 0;

            while byte_pos < lhs.len() {
                if !lhs.as_bytes()[byte_pos].eq_ignore_ascii_case(&rhs.as_bytes()[byte_pos]) {
                    return false;
                }
                byte_pos += 1;
            }

            true
        }

        if eq_ignore_ascii_case(filter, "off") {
            Some(LevelFilter::Off)
        } else if eq_ignore_ascii_case(filter, "error") {
            Some(LevelFilter::Error)
        } else if eq_ignore_ascii_case(filter, "warn") {
            Some(LevelFilter::Warn)
        } else if eq_ignore_ascii_case(filter, "info") {
            Some(LevelFilter::Info)
        } else if eq_ignore_ascii_case(filter, "debug") {
            Some(LevelFilter::Debug)
        } else if eq_ignore_ascii_case(filter, "trace") {
            Some(LevelFilter::Trace)
        } else {
            None
        }
    }
}
