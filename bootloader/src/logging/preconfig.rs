//! Logging configuration before reading configuration file.

use log::LevelFilter;

/// The default level for pre-config logging.
const DEFAULT_LEVEL: LevelFilter = LevelFilter::Error;

/// Configuration of the [`GLOBAL_FILTER`][f] before reading the configuration file.
///
/// [f]: super::GLOBAL_FILTER
pub const PRECONFIG_GLOBAL: LevelFilter = match core::option_env!("PRECONFIG_GLOBAL") {
    Some(env) => match level_filter_from_str(env) {
        Some(filter) => filter,
        None => panic!(
            "`PRECONFIG_GLOBAL` must be either `off`, `error`, `warn`, `info`, `debug`, or `trace`"
        ),
    },
    None => DEFAULT_LEVEL,
};

/// Configuration of the [`SERIAL_FILTER`][f] before reading the configuration file.
///
/// [f]: super::SERIAL_FILTER
#[cfg(feature = "serial_logging")]
pub const PRECONFIG_SERIAL: LevelFilter = match core::option_env!("PRECONFIG_SERIAL") {
    Some(env) => match level_filter_from_str(env) {
        Some(filter) => filter,
        None => panic!(
            "`PRECONFIG_SERIAL` must be either `off`, `error`, `warn`, `info`, `debug`, or `trace`"
        ),
    },
    None => DEFAULT_LEVEL,
};

/// Configuration of the [`FRAMEBUFFER_FILTER`][f] before reading the configuration file.
///
/// [f]: super::FRAMEBUFFER_FILTER
#[cfg(feature = "framebuffer_logging")]
pub const PRECONFIG_FRAMEBUFFER: LevelFilter = match core::option_env!("PRECONFIG_FRAMEBUFFER") {
    Some(env) => match level_filter_from_str(env) {
        Some(filter) => filter,
        None => panic!(
            "`PRECONFIG_FRAMEBUFFER` must be either `off`, `error`, `warn`, `info`, `debug`, or `trace`"
        ),
    },
    None => DEFAULT_LEVEL,
};

/// Parses a [`LevelFilter`] from a string.
const fn level_filter_from_str(filter: &str) -> Option<LevelFilter> {
    /// Const string comparision ignoring ascii case.
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
