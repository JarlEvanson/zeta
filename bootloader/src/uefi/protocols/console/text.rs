//! Rust-y wrappers around text input/output protocols.

use core::{mem::MaybeUninit, ptr::NonNull};

use uefi::{
    datatypes::{CStr16, Status},
    protocols::console::text::{
        BackgroundColor, ForegroundColor, SimpleTextOutputMode, SimpleTextOutputProtocol,
    },
};

use crate::uefi::protocols::Protocol;

/// Interface for text-based output devices.
pub struct SimpleTextOutput {
    /// Pointer to the interface.
    ptr: NonNull<SimpleTextOutputProtocol>,
}

impl SimpleTextOutput {
    /// Writes the provided `str` to the output device.
    ///
    /// The [`CStr16`] is displayed at the current cursor location
    /// on the output device(s) and the cursor is advanced according to listed rules.
    pub fn output_string(&mut self, str: &CStr16) -> Result<(), Status> {
        // SAFETY:
        // All [`SimpleTextOutput`] structures point to a valid [`SimpleTextOutputProtocol`].
        let output_string_ptr = unsafe { (*self.ptr.as_ptr()).output_string };

        // SAFETY:
        // `output_string_ptr` is being called with valid arguments and the correct interface.
        unsafe { output_string_ptr(self.ptr.as_ptr(), str.as_ptr()) }.into_result()
    }

    /// Returns information for an available text mode that the output device(s) supports.
    ///
    /// It is required that all output devices support at least 80x20 text mode, which is defined
    /// to be mode 0. If the output devices support 80x50, that is defined to be mode 1. All other
    /// text dimensions supported by the device will follow as modes 2 and above.
    pub fn query_mode(&self, mode: usize) -> Result<Dimensions, Status> {
        // SAFETY:
        // All [`SimpleTextOutput`] structures point to a valid [`SimpleTextOutputProtocol`].
        let query_mode_ptr = unsafe { (*self.ptr.as_ptr()).query_mode };

        let mut rows = MaybeUninit::uninit();
        let mut columns = MaybeUninit::uninit();

        // SAFETY:
        // `query_mode_ptr` is being called with valid arguments and the correct interface.
        unsafe { query_mode_ptr(self.ptr.as_ptr(), mode, &mut columns, &mut rows) }
            .into_result()
            .map(|()| Dimensions {
                // SAFETY:
                // `result` is [`Status::SUCCESS`], so `columns` has been initialized.
                columns: unsafe { columns.assume_init() },
                // SAFETY:
                // `result` is [`Status::SUCCESS`], so `rows` has been initialized.
                rows: unsafe { rows.assume_init() },
            })
    }

    /// Sets the output device(s) to a specified `mode`.
    ///
    /// On success, the device is in the [`Dimensions`] for the requested mode, and the device has been cleared
    /// to the current background color with the cursor at (0, 0).
    pub fn set_mode(&mut self, mode: usize) -> Result<(), Status> {
        // SAFETY:
        // All [`SimpleTextOutput`] structures point to a valid [`SimpleTextOutputProtocol`].
        let set_mode_ptr = unsafe { (*self.ptr.as_ptr()).set_mode };

        // SAFETY:
        // `set_mode_ptr` is being called with valid arguments and the correct interface.
        unsafe { set_mode_ptr(self.ptr.as_ptr(), mode) }.into_result()
    }

    /// Sets the background and foreground colors for the [`SimpleTextOutput::output_string()`] function.
    pub fn set_attribute(
        &mut self,
        background: BackgroundColor,
        foreground: ForegroundColor,
    ) -> Result<(), Status> {
        // SAFETY:
        // All [`SimpleTextOutput`] structures point to a valid [`SimpleTextOutputProtocol`].
        let set_attribute_ptr = unsafe { (*self.ptr.as_ptr()).set_attribute };

        // SAFETY:
        // `set_attribute_ptr` is being called with valid arguments and the correct interface.
        unsafe {
            set_attribute_ptr(
                self.ptr.as_ptr(),
                ((background as usize) << 4) | foreground as usize,
            )
        }
        .into_result()
    }

    /// Makes the cursor visible or invisible.
    pub fn enable_cursor(&mut self, visible: bool) -> Result<(), Status> {
        // SAFETY:
        // All [`SimpleTextOutput`] structures point to a valid [`SimpleTextOutputProtocol`].
        let enable_cursor_ptr = unsafe { (*self.ptr.as_ptr()).enable_cursor };

        // SAFETY:
        // `enable_cursor_ptr` is being called with valid arguments and the correct interface.
        unsafe { enable_cursor_ptr(self.ptr.as_ptr(), visible) }.into_result()
    }

    /// Returns a read-only view of various information about the state of the [`SimpleTextOutput`] device.
    pub fn info(&self) -> &SimpleTextOutputMode {
        // SAFETY:
        // All [`SimpleTextOutput`] structures point to a valid [`SimpleTextOutputProtocol`].
        let mode_ptr = unsafe { (*self.ptr.as_ptr()).mode };

        // SAFETY:
        // All [`SimpleTextOutput`] structures point to a valid [`SimpleTextOutputProtocol`].
        unsafe { &*mode_ptr }
    }
}

/// Dimensions of a [`SimpleTextOutput`] device.
pub struct Dimensions {
    /// The number of columns of glyphs the mode supports.
    pub columns: usize,
    /// The number of rows of glyphs the mode supports.
    pub rows: usize,
}

impl Protocol for SimpleTextOutput {
    const GUID: uefi::datatypes::Guid = SimpleTextOutputProtocol::GUID;

    unsafe fn from_ffi_ptr(ptr: *const core::ffi::c_void) -> Self {
        SimpleTextOutput {
            ptr: NonNull::new(ptr.cast::<SimpleTextOutputProtocol>().cast_mut()).unwrap(),
        }
    }
}
