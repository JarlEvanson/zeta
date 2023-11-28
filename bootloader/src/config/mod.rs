use digest::sha512::Digest;
use log::LevelFilter;

use crate::vec::Vec;

mod parser;

pub use parser::parse_configuration_file;

/// A parsed TOML configuration file.
pub struct Config {
    /// If true, then all unused memory will be randomized.
    randomize_memory: bool,
    /// Desired [`LevelFilter`] state for logging.
    pub logging: LoggingFilters,
    /// Information regarding the kernel file and the requested modules to load.
    pub kernel: Kernel<'static>,
    /// Information regarding the known modules.
    pub modules: Vec<Module<'static>>,
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
pub struct Kernel<'config> {
    /// Path to kernel executable file.
    pub path: &'config str,
    /// SHA-512 digest of kernel executable file.
    pub digest: Digest,
    /// Arguments to be passed to the module.
    pub args: Vec<&'config str>,
    /// Modules to be loaded with the kernel.
    pub loaded_modules: Vec<&'config str>,
}

/// Information concerning a module.
pub struct Module<'config> {
    /// Name of the module.
    pub name: &'config str,
    /// Path to the module executable file.
    pub path: &'config str,
    /// SHA-512 digest of the module executable file.
    pub digest: Digest,
    /// Arguments to be passed to the module.
    pub args: Vec<&'config str>,
}
