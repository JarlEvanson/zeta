//! Helper functions for parsing items from strings. This is meant to be used for compile time
//! for configuration.

/// Attempts to acquire the environment variable `config_name`, returning `default` if the
/// variable is not set. If the variable is set and the parsing of the variable fails,
/// this reports an error.
#[macro_export]
macro_rules! parse_or_default {
    ($default:expr, $config_name:literal, $parse:path) => {{
        match option_env!($config_name) {
            Some(s) => {
                if let Some(value) = $parse(s) {
                    value
                } else {
                    panic!(concat!($config_name, " received an invalid input"));
                }
            }
            None => $default,
        }
    }};
}

/// Parses a [`bool`] from `s`.
pub const fn parse_bool(s: &[u8]) -> Option<bool> {
    #[allow(clippy::missing_docs_in_private_items)]
    const TRUE: &[u8] = "true".as_bytes();
    #[allow(clippy::missing_docs_in_private_items)]
    const FALSE: &[u8] = "false".as_bytes();

    match s {
        TRUE => Some(true),
        FALSE => Some(false),
        _ => None,
    }
}

/// Parses a [`u8`] from `s`, returning [`None`] if it would overflow.
pub const fn parse_u8(s: &[u8]) -> Option<u8> {
    let mut value: u8 = 0;
    let mut index = 0;

    while index < s.len() {
        if s[0] < b'0' || s[0] > b'9' {
            return None;
        }

        if let Some(new_value) = value.checked_mul(10) {
            value = new_value;
        } else {
            return None;
        };

        let Some(new_value) = value.checked_add(s[0] - b'0') else {
            return None;
        };

        value = new_value;
        index += 1;
    }

    Some(value)
}

/// Parses a [`u16`] from `s`, returning [`None`] if it would overflow.
pub const fn parse_u16(s: &[u8]) -> Option<u16> {
    let mut value: u16 = 0;
    let mut index = 0;

    while index < s.len() {
        if s[0] < b'0' || s[0] > b'9' {
            return None;
        }

        if let Some(new_value) = value.checked_mul(10) {
            value = new_value;
        } else {
            return None;
        };

        let Some(new_value) = value.checked_add((s[0] - b'0') as u16) else {
            return None;
        };

        value = new_value;
        index += 1;
    }

    Some(value)
}

/// Parses a [`u32`] from `s`, returning [`None`] if it would overflow.
pub const fn parse_u32(s: &[u8]) -> Option<u32> {
    let mut value: u32 = 0;
    let mut index = 0;

    while index < s.len() {
        if s[0] < b'0' || s[0] > b'9' {
            return None;
        }

        if let Some(new_value) = value.checked_mul(10) {
            value = new_value;
        } else {
            return None;
        };

        let Some(new_value) = value.checked_add((s[0] - b'0') as u32) else {
            return None;
        };

        value = new_value;
        index += 1;
    }

    Some(value)
}

/// Parses a [`u64`] from `s`, returning [`None`] if it would overflow.
pub const fn parse_u64(s: &[u8]) -> Option<u64> {
    let mut value: u64 = 0;
    let mut index = 0;

    while index < s.len() {
        if s[0] < b'0' || s[0] > b'9' {
            return None;
        }

        if let Some(new_value) = value.checked_mul(10) {
            value = new_value;
        } else {
            return None;
        };

        let Some(new_value) = value.checked_add((s[0] - b'0') as u64) else {
            return None;
        };

        value = new_value;
        index += 1;
    }

    Some(value)
}

/// Parses a [`usize`] from `s`, returning [`None`] if it would overflow.
pub const fn parse_usize(s: &[u8]) -> Option<usize> {
    let mut value: usize = 0;
    let mut index = 0;

    while index < s.len() {
        if s[0] < b'0' || s[0] > b'9' {
            return None;
        }

        if let Some(new_value) = value.checked_mul(10) {
            value = new_value;
        } else {
            return None;
        };

        let Some(new_value) = value.checked_add((s[0] - b'0') as usize) else {
            return None;
        };

        value = new_value;
        index += 1;
    }

    Some(value)
}
