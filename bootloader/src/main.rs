//! A bootloader for the zeta microkernel.

#![no_std]
#![cfg_attr(not(test), no_main)]
#![feature(lint_reasons, maybe_uninit_slice, strict_provenance, error_in_core)]

use digest::sha512::Digest;
use filesystem::{acquire_boot_partition_root_directory, load_file, AcquireRootError};
use uefi::{
    boot::acquire_boot_handle,
    table::{Boot, SystemTable},
    CStr16, Handle, Status,
};

use crate::config::parse_configuration_file;

mod config;
mod filesystem;
mod logging;
mod vec;

/// The default logging level.
pub const DEFAULT_LOGGING_LEVEL: log::LevelFilter = log::LevelFilter::Info;

/// The path to the configuration file.
const CONFIG_PATH: &CStr16 = uefi::cstr16!("zeta\\config.toml");

/// The current digest of the test configuration file.
#[export_name = "digest"]
#[link_section = ".config"]
static CONFIG_DIGEST: Digest = Digest::from_u64s([
    0x0340_f948_f096_c1cb,
    0xd1b7_3efd_a6f5_4a49,
    0xc28b_8bd9_397a_ba28,
    0x5abf_8552_293e_dde6,
    0x4fc3_763a_0db9_cad7,
    0x53bd_1e19_03a7_6d65,
    0x18de_de79_36ae_3896,
    0xe699_4aad_8db6_4eb3,
]);

/// The amount of time we stall before returning in the event of an error.
const ERROR_STALL_TIME: usize = 10_000_000;

#[uefi_macros::entry]
fn main(image_handle: Handle, system_table: SystemTable<Boot>) -> Status {
    match logging::initialize() {
        Ok(()) => log::info!(target: "logging", "logging initialized"),
        Err(_err) => {
            acquire_boot_handle().stall(ERROR_STALL_TIME);
            return Status::ABORTED;
        }
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

    log::info!(target: "config", "parsing configuration file");
    parse_configuration_file(config_string).unwrap();

    // let Ok(config) = Config::parse(config_string) else {
    //     log::error!("error parsing config file: ");
    //     acquire_boot_handle().stall(ERROR_STALL_TIME);
    //     return Status::INVALID_LANGUAGE;
    // };

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
