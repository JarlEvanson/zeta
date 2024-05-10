//! Kernel for the zeta project.

#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

pub mod arch;
pub mod cells;
pub mod polyfill;
pub mod spinlock;
pub mod utils;

#[cfg(test)]
fn main() {}

/// Function that handles panics.
#[cfg_attr(not(test), panic_handler)]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
