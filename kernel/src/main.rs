//! Kernel for the zeta project.

#![no_std]
#![no_main]

pub mod arch;

/// Function that handles panics.
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
