//! A bootloader for the zeta microkernel.

#![no_std]
#![no_main]
#![feature(lint_reasons, maybe_uninit_slice, strict_provenance)]

use digest::sha512::Digest;
use filesystem::{
    acquire_boot_partition_root_directory, load_file, AcquireRootError, LoadFileError,
};
use uefi::{
    table::{Boot, SystemTable},
    CStr16, Handle, Status,
};

pub mod filesystem;
pub mod logging;
pub mod vec;

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

#[uefi_macros::entry]
fn main(image_handle: Handle, system_table: SystemTable<Boot>) -> Status {
    match logging::initialize() {
        Ok(()) => log::info!(target: "logging", "logging initialized"),
        Err(_err) => return Status::ABORTED,
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
            return Status::LOAD_ERROR;
        }
    };

    log::info!(target: "config", "loading configuration file");
    let result = match load_file(&mut root_dir, CONFIG_PATH, CONFIG_DIGEST) {
        Ok(bytes) => bytes,
        Err(LoadFileError::AccessDenied) => {
            log::error!("access to file was not allowed");
            return Status::ACCESS_DENIED;
        }
        Err(LoadFileError::InvalidDigest) => {
            log::error!("the digest of the configuration file was unexpected: verify authenticity and then embed its hash");
            return Status::SECURITY_VIOLATION;
        }
        Err(LoadFileError::MediaError) => {
            log::error!("a media error occurred");
            return Status::ABORTED;
        }
        Err(LoadFileError::NotFile) => {
            log::error!("item at \"{CONFIG_PATH}\" was not a file");
            return Status::INVALID_PARAMETER;
        }
        Err(LoadFileError::NotFound) => {
            log::error!("a config file must exist");
            return Status::NOT_FOUND;
        }
        Err(LoadFileError::OutOfResources) => {
            log::error!("out of resources");
            return Status::OUT_OF_RESOURCES;
        }
        Err(LoadFileError::VolumeCorrupted) => {
            log::error!("the volume was corrupted");
            return Status::VOLUME_CORRUPTED;
        }
    };

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
