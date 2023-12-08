use digest::sha512::Digest;
use log::LevelFilter;

use crate::vec::Vec;

mod parser;

pub use parser::parse_configuration_file;

/// A parsed TOML configuration file.
pub struct Config {
    /// If true, then all unused memory will be randomized.
    pub randomize_memory: bool,
    /// Desired [`LevelFilter`] state for logging.
    pub logging: LoggingFilters,
    /// Information regarding the kernel file and the requested modules to load.
    pub kernel: Kernel,
    /// Information regarding the known modules.
    pub modules: Vec<Module>,
    pub strings: StringStorage,
}

/// Potential settings for the global filter of log messages, the filter on all logs
/// outputted through the serial port, and the filter on all logs outputted onto the
/// framebuffer.
pub struct LoggingFilters {
    /// The setting of the global filter.
    pub global: LevelFilter,
    /// The setting of the serial filter.
    pub serial: LevelFilter,
    /// The setting of the framebuffer filter.
    pub framebuffer: LevelFilter,
}

/// Information concerning the kernel file and modules to load.
pub struct Kernel {
    /// Path to kernel executable file.
    pub path: StringHandle,
    /// SHA-512 digest of kernel executable file.
    pub checksum: Digest,
    /// Arguments to be passed to the module.
    pub args: Vec<StringHandle>,
    /// Modules to be loaded with the kernel.
    pub loaded_modules: Vec<StringHandle>,
}

/// Information concerning a module.
pub struct Module {
    /// Name of the module.
    pub name: StringHandle,
    /// Path to the module executable file.
    pub path: StringHandle,
    /// SHA-512 digest of the module executable file.
    pub digest: Digest,
    /// Arguments to be passed to the module.
    pub args: Vec<StringHandle>,
}

pub struct StringStorage {
    storage: Vec<u8>,
}

impl StringStorage {
    pub fn new() -> StringStorage {
        Self {
            storage: Vec::new(),
        }
    }

    pub fn add_str_from_chars<I: Iterator<Item = char> + Clone>(
        &mut self,
        iter: I,
    ) -> Result<StringHandle, ()> {
        let byte_count = iter.clone().map(|c| c.len_utf8()).count();

        let start = self.storage.len();

        self.storage.try_reserve(byte_count).map_err(|_| ())?;

        for c in iter.map(CharByteIter::new).flatten() {
            assert!(self.storage.push_within_capacity(c).is_ok());
        }

        let handle = StringHandle {
            start,
            len: byte_count,
        };

        Ok(handle)
    }

    pub fn lookup(&self, handle: StringHandle) -> &str {
        core::str::from_utf8(&self.storage.as_slice()[handle.start..(handle.start + handle.len)])
            .unwrap()
    }
}

pub struct StringHandle {
    start: usize,
    len: usize,
}

struct CharByteIter {
    len_utf8: usize,
    iter: core::array::IntoIter<u8, 4>,
}

impl CharByteIter {
    fn new(c: char) -> CharByteIter {
        let mut k = [0; 4];
        c.encode_utf8(&mut k);

        Self {
            len_utf8: c.len_utf8(),
            iter: k.into_iter(),
        }
    }
}

impl Iterator for CharByteIter {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        let bytes_consumed = 4 - self.iter.len();
        self.iter.next().and_then(|bytes| {
            if bytes_consumed < self.len_utf8 {
                Some(bytes)
            } else {
                None
            }
        })
    }
}
