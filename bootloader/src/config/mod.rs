//! Configuration for the bootloader.
//!
//! Contains a TOML parser specifically for the config file.

use core::fmt::Debug;

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
    /// Storage for all parsed strings.
    pub strings: StringStorage,
}

/// Potential settings for the global filter of log messages, the filter on all logs
/// outputted through the serial port, and the filter on all logs outputted onto the
/// framebuffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoggingFilters {
    /// The setting of the global filter.
    pub global: LevelFilter,
    /// The setting of the serial filter.
    pub serial: LevelFilter,
    /// The setting of the framebuffer filter.
    pub framebuffer: LevelFilter,
}

/// Information concerning the kernel file and modules to load.
#[derive(Debug)]
pub struct Kernel {
    /// Path to kernel executable file.
    pub path: StringHandle,
    /// SHA-512 digest of kernel executable file.
    pub checksum: Digest,
    /// Arguments to be passed to the kernel.
    pub args: Vec<StringHandle>,
}

/// Information concerning a module.
#[derive(Debug)]
pub struct Module {
    /// Path to the module executable file.
    pub path: StringHandle,
    /// SHA-512 digest of the module executable file.
    pub checksum: Digest,
    /// Arguments to be passed to the module.
    pub args: Vec<StringHandle>,
}

impl Debug for Config {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut dstruct = f.debug_struct("Config");

        dstruct.field("randomize_memory", &self.randomize_memory);
        dstruct.field("logging", &self.logging);

        dstruct.field_with("kernel", |f| {
            let mut dstruct = f.debug_struct("Kernel");

            dstruct.field("path", &self.strings.lookup(self.kernel.path));
            dstruct.field("checksum", &self.kernel.checksum);

            dstruct.field_with("args", |f| {
                let mut dlist = f.debug_list();

                for arg in self.kernel.args.as_slice().iter().copied() {
                    dlist.entry(&self.strings.lookup(arg));
                }

                dlist.finish()
            });

            dstruct.finish()
        });

        dstruct.field_with("modules", |f| {
            let mut dlist = f.debug_list();

            for module in self.modules.as_slice() {
                dlist.entry_with(|f| {
                    let mut dstruct = f.debug_struct("Module");

                    dstruct.field("path", &self.strings.lookup(module.path));
                    dstruct.field("checksum", &module.checksum);

                    dstruct.field_with("args", |f| {
                        let mut dlist = f.debug_list();

                        for arg in module.args.as_slice().iter().copied() {
                            dlist.entry(&self.strings.lookup(arg));
                        }

                        dlist.finish()
                    });

                    dstruct.finish()
                });
            }

            dlist.finish()
        });

        dstruct.field_with("strings", |f| f.write_str("StringStorage"));

        dstruct.finish_non_exhaustive()
    }
}

/// A compact storage for [`str`]s.
pub struct StringStorage {
    /// The underlying storage for the bytes making up the stored [`str`]s.
    storage: Vec<u8>,
}

impl StringStorage {
    /// Creates a new [`StringStorage`].
    pub fn new() -> StringStorage {
        Self {
            storage: Vec::new(),
        }
    }

    /// Adds a [`str`] produced from `iter` to the [`StringStorage`] and returns
    /// its handle.
    pub fn add_str_from_chars<I: Iterator<Item = char> + Clone>(
        &mut self,
        iter: I,
    ) -> Result<StringHandle, ()> {
        let byte_count = iter.clone().map(char::len_utf8).sum::<usize>();

        let start = self.storage.len();

        self.storage.try_reserve(byte_count).map_err(|_| ())?;

        for c in iter.flat_map(CharByteIter::new) {
            assert!(self.storage.push_within_capacity(c).is_ok());
        }

        let handle = StringHandle {
            start,
            len: byte_count,
        };

        Ok(handle)
    }

    /// Retrives the [`str`] associated with `handle`.
    ///
    /// # Panics
    /// Panics if `handle` does not beong to this [`StringStorage`].
    pub fn lookup(&self, handle: StringHandle) -> &str {
        core::str::from_utf8(&self.storage.as_slice()[handle.start..(handle.start + handle.len)])
            .unwrap()
    }
}

/// A handle to a [`str`] stored in a [`StringStorage`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StringHandle {
    /// The byte index of the start of the [`str`].
    start: usize,
    /// The length of the [`str`].
    len: usize,
}

/// An iterator over the bytes of a utf-8 encoded codepoint.
struct CharByteIter {
    /// The length of the codepoint.
    len_utf8: usize,
    /// The byte iterator.
    iter: core::array::IntoIter<u8, 4>,
}

impl CharByteIter {
    /// Creates a new [`CharByteIter`] from `c`.
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
