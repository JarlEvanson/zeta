//! Bootloader for the zeta project.

#![no_std]
#![no_main]

#[export_name = "efi_main"]
fn efi_main(handle: *mut core::ffi::c_void, system_table: *mut ()) -> usize {
    0
}

/// Handles panics occurring while booting the system.
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}