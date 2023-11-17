#![no_std]
#![no_main]

use uefi::{Handle, table::{SystemTable, Boot}, Status};

#[uefi_macros::entry]
fn main(image_handle: Handle, system_table: SystemTable<Boot>) -> Status {
    Status::SUCCESS
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! { 
    loop {
        core::hint::spin_loop();
    }
}