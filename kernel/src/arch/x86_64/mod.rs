//! Interfaces and implementations of x86_64 specific functionality.

use core::{fmt::Write, marker::PhantomData};

use crate::logging::Logger;

pub mod boot;

/// Logs to the QEMU debug connection port.
///
/// Unsafe to use if not running under QEMU.
struct DebugConLogger(PhantomData<()>);

impl DebugConLogger {
    /// The port to which QEMU debugcon is connected.
    pub const OUTPUT_PORT: u16 = 0xE9;

    /// Creates a new [`DebugConLogger`].
    ///
    /// # Safety
    /// Throughout the lifetime of the created [`DebugConLogger`], exclusive access to the port at
    /// address [`DebugConLogger::OUTPUT_PORT`] must be had.
    pub const unsafe fn new() -> DebugConLogger {
        DebugConLogger(PhantomData)
    }

    /// Creates a new &mut [`DebugConLogger`].
    ///
    /// # Safety
    /// Throughout the lifetime of the created &mut [`DebugConLogger`], exclusive access to the port at
    /// address [`DebugConLogger::OUTPUT_PORT`] must be had.
    pub unsafe fn new_mut<'a>() -> &'a mut DebugConLogger {
        // SAFETY:
        // Zero-sized mutable references are safe.
        unsafe { &mut *core::ptr::dangling_mut() }
    }

    /// Writes `msg` to the QEMU debug connection.
    pub fn write_bytes(&mut self, msg: &[u8]) {
        // SAFETY:
        // Since `self` is alive, exclusive access to [`DebugConLogger::OUTPUT_PORT`] is held.
        unsafe {
            core::arch::asm!(
                "rep outsb",
                in("dx") DebugConLogger::OUTPUT_PORT,
                inout("rsi") msg.as_ptr() => _,
                inout("rcx") msg.len() => _,
                options(readonly, nostack, preserves_flags)
            );
        }
    }
}

impl Write for DebugConLogger {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_bytes(s.as_bytes());

        Ok(())
    }
}

impl Logger for DebugConLogger {
    fn log(&mut self, level: crate::logging::LogLevel, args: core::fmt::Arguments) {
        let _ = write!(self, "[{:?}]: {}", level, args);
    }
}
