//! The kernel for zeta.

#![no_std]
#![no_main]

mod const_parsing;

#[export_name = "_start"]
extern "C" fn main() -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
