//! Bootloader for the zeta project.

#![no_std]
#![no_main]

use crate::uefi::tables::system::{Boot, SystemTable};
use ::uefi::{
    datatypes::Status,
    protocols::console::text::{BackgroundColor, ForegroundColor},
};

mod uefi;

entry_point!(entry_point);

/// The main logic for the bootloader.
fn entry_point(mut system_table: SystemTable<Boot>) -> Status {
    setup_outputs(&mut system_table);

    system_table.boot_services().stall(10_000_000);

    Status::SUCCESS
}

/// Sets up console out and console err for the executable.
fn setup_outputs(system_table: &mut SystemTable<Boot>) {
    let mut console_out = system_table.console_out();

    let max_mode = console_out.info().max_mode as usize;

    let mut best_mode = 0;

    let mut best_rows = 0;
    let mut best_columns = 0;

    for mode in 0..max_mode {
        let Ok(dimensions) = console_out.query_mode(mode) else {
            continue;
        };

        if dimensions.rows > best_rows || dimensions.columns > best_columns {
            best_mode = mode;

            best_rows = dimensions.rows;
            best_columns = dimensions.columns;
        }
    }

    let _ = console_out.set_mode(best_mode);
    let _ = console_out.enable_cursor(false);
    let _ = console_out.set_attribute(BackgroundColor::Black, ForegroundColor::Green);

    let mut console_err = system_table.console_err();

    let max_mode = console_err.info().max_mode as usize;

    let mut best_mode = 0;

    let mut best_rows = 0;
    let mut best_columns = 0;

    for mode in 0..max_mode {
        let Ok(dimensions) = console_err.query_mode(mode) else {
            continue;
        };

        if dimensions.rows > best_rows || dimensions.columns > best_columns {
            best_mode = mode;

            best_rows = dimensions.rows;
            best_columns = dimensions.columns;
        }
    }

    let _ = console_err.set_mode(best_mode);
    let _ = console_err.enable_cursor(false);
    let _ = console_err.set_attribute(BackgroundColor::Black, ForegroundColor::Red);
}

/// Handles panics occurring while booting the system.
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
