//! Bootloader for the zeta project.

#![no_std]
#![no_main]

use crate::uefi::tables::system::{Boot, SystemTable};
use ::uefi::datatypes::Status;

mod uefi;

entry_point!(entry_point);

/// The main logic for the bootloader.
fn entry_point(mut system_table: SystemTable<Boot>) -> Status {
    system_table.boot_services().stall(10_000_000);

    Status::SUCCESS
}

/// Handles panics occurring while booting the system.
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
