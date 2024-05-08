//! Kernel for the zeta project.

#![no_std]
#![no_main]

pub mod arch;
pub mod cells;
pub mod utils;

/// Function that handles panics.
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
