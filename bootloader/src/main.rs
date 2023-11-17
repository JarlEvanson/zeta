#![no_std]
#![no_main]
#![feature(lint_reasons)]

use filesystem::{acquire_boot_partition_root_directory, AcquireRootError};
use uefi::{
    table::{Boot, SystemTable},
    Handle, Status,
};

pub mod filesystem;
pub mod logging;

#[uefi_macros::entry]
fn main(image_handle: Handle, system_table: SystemTable<Boot>) -> Status {
    match logging::initialize() {
        Ok(()) => log::info!(target: "logging", "logging initialized"),
        Err(_err) => return Status::ABORTED,
    }

    let root_dir = match acquire_boot_partition_root_directory() {
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

    Status::SUCCESS
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error!(target: "panic", "{info}");

    loop {
        core::hint::spin_loop();
    }
}
