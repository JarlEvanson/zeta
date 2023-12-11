//! A bootloader for the zeta microkernel.

#![no_std]
#![cfg_attr(not(test), no_main)]
#![feature(
    lint_reasons,
    maybe_uninit_slice,
    strict_provenance,
    error_in_core,
    debug_closure_helpers
)]

use digest::sha512::Digest;
use filesystem::{acquire_boot_partition_root_directory, load_file, AcquireRootError};
use uefi::{
    boot::acquire_boot_handle,
    table::{Boot, SystemTable},
    CStr16, Handle, Status,
};

use crate::{
    config::parse_configuration_file,
    logging::{set_framebuffer_filter, set_global_filter, set_serial_filter},
    vec::Vec,
};

mod config;
mod filesystem;
mod logging;
#[cfg_attr(not(feature = "framebuffer_logging"), expect(unused))]
mod terminal;
mod vec;

/// The default logging level.
pub const DEFAULT_LOGGING_LEVEL: log::LevelFilter = log::LevelFilter::Info;

/// The path to the configuration file.
const CONFIG_PATH: &CStr16 = uefi::cstr16!("zeta\\config.toml");

/// The current digest of the test configuration file.
#[export_name = "digest"]
#[link_section = ".config"]
static CONFIG_DIGEST: Digest = match Digest::from_str(
    "cf5630143de2f453c7758a5977676d485ecbf10ac44af0dee37e36e0c072330e1667ed146adb4ab3015bbe2198560047b07a7dd5e32377cc11dca5fd2d2b3996"
) {
    Some(digest) => digest,
    None => panic!("invalid digest"),
};

/// The amount of time we stall before returning in the event of an error.
const ERROR_STALL_TIME: usize = 10_000_000;

#[uefi_macros::entry]
fn main(image_handle: Handle, system_table: SystemTable<Boot>) -> Status {
    if let Err(err) = logging::initialize() {
        return err;
    }

    let mut root_dir = match acquire_boot_partition_root_directory() {
        Ok(root_dir) => {
            log::trace!("acquired root directory of boot partition");
            root_dir
        }
        Err(err) => {
            match err {
                AcquireRootError::InvalidBootMethod => {
                    log::error!("bootloader was loaded using an unsupported method");
                }
                AcquireRootError::InvalidVolume => {
                    log::error!("failed to open the volume from which the bootloader was loaded");
                }
            }
            acquire_boot_handle().stall(ERROR_STALL_TIME);
            return Status::LOAD_ERROR;
        }
    };

    log::info!(target: "config", "loading configuration file");
    let result = match load_file(&mut root_dir, CONFIG_PATH, CONFIG_DIGEST) {
        Ok(bytes) => bytes,
        Err(err) => {
            log::error!("{}", err);
            acquire_boot_handle().stall(ERROR_STALL_TIME);
            return Into::<Status>::into(err);
        }
    };

    #[allow(unused)]
    let Ok(config_string) = core::str::from_utf8(result.as_slice()) else {
        log::error!("config file must be utf-8");
        acquire_boot_handle().stall(ERROR_STALL_TIME);
        return Status::INVALID_LANGUAGE;
    };

    let Ok(config) = parse_configuration_file(config_string) else {
        log::error!("error parsing config file: ");
        acquire_boot_handle().stall(ERROR_STALL_TIME);
        return Status::INVALID_LANGUAGE;
    };
    log::info!(target: "config", "configuration file parsed");
    log::debug!(target: "config", "{:#?}", config);

    set_global_filter(config.logging.global);
    set_serial_filter(config.logging.serial);
    set_framebuffer_filter(config.logging.framebuffer);

    let kernel_path = config.paths.lookup(config.kernel.path);
    log::info!("loading kernel file from {}", kernel_path);
    let kernel_bytes = match load_file(&mut root_dir, kernel_path, config.kernel.checksum) {
        Ok(bytes) => bytes,
        Err(err) => {
            log::error!(target: "filesystem", "{err}");
            acquire_boot_handle().stall(ERROR_STALL_TIME);
            return Into::<Status>::into(err);
        }
    };
    log::info!("kernel file loaded and verified");

    let mut modules = match Vec::with_capacity(config.modules.len()) {
        Ok(modules) => modules,
        Err(err) => {
            log::error!(target: "memory", "{err}");
            acquire_boot_handle().stall(ERROR_STALL_TIME);
            return Status::OUT_OF_RESOURCES;
        }
    };

    for module in config.modules.as_slice() {
        let module_bytes = match load_file(
            &mut root_dir,
            config.paths.lookup(module.path),
            module.checksum,
        ) {
            Ok(bytes) => bytes,
            Err(err) => {
                log::error!(target: "filesystem", "{err}");
                acquire_boot_handle().stall(ERROR_STALL_TIME);
                return Into::<Status>::into(err);
            }
        };
        assert!(modules.push_within_capacity(module_bytes).is_ok());
    }
    log::info!(target: "filesystem", "all modules loaded");

    logging::prepare_to_exit_boot_services();

    loop {
        core::hint::spin_loop();
    }
}

/// The panic handler for the bootloader.
///
/// Logs the panic message and then spins.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error!(target: "panic", "{info}");

    loop {
        core::hint::spin_loop();
    }
}
