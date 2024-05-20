//! Protocols used to support a simple text-based console.

use core::mem::MaybeUninit;

use crate::datatypes::{Char16, Guid, Status};

/// A protocol used to control text-based devices.
///
/// THe minimum supported text mode of devices that support the [`SimpleTextOutputProtocol`]
/// is 80x25 characters.
#[repr(C)]
pub struct SimpleTextOutputProtocol {
    /// Reset the device associated with this [`SimpleTextOutputProtocol`].
    ///
    /// If `extended_verification` is `true`, then the driver may perform a more exhaustive
    /// verification of the operation of the device.
    pub reset: unsafe extern "efiapi" fn(this: *mut Self, extended_verification: bool) -> Status,
    /// Displays `str` on the device associated with this [`SimpleTextOutputProtocol`]
    /// at the current cursor location.
    pub output_string: unsafe extern "efiapi" fn(this: *mut Self, str: *const Char16) -> Status,
    /// Tests to see if the device associated with this [`SimpleTextOutputProtocol`]
    /// supports `str`.
    pub test_string: unsafe extern "efiapi" fn(this: *mut Self, str: *const Char16) -> Status,
    /// Queries information concerning the supported text modes of the device associated with
    /// this [`SimpleTextOutputProtocol`].
    ///
    /// Returns the number of columns supported by `mode` in `columns`, and the number of rows in `rows`.
    pub query_mode: unsafe extern "efiapi" fn(
        this: *mut Self,
        mode: usize,
        columns: *mut MaybeUninit<usize>,
        rows: *mut MaybeUninit<usize>,
    ) -> Status,
    /// Sets the current mode of the device associated with the [`SimpleTextOutputProtocol`] to `mode`.
    pub set_mode: unsafe extern "efiapi" fn(this: *mut Self, mode: usize) -> Status,
    /// Sets the foreground and background colors of the text that is outputted.
    ///
    /// Bits 0..=3 control the foreground color, and bits 4..=6 control the background color.
    pub set_attribute: unsafe extern "efiapi" fn(this: *mut Self, attribute: usize) -> Status,
    /// Clears the screen with the currently set background color. The cursor position
    /// is set to (0, 0).
    pub clear_screen: unsafe extern "efiapi" fn(this: *mut Self) -> Status,
    /// Sets the current cursor position to `(column, row)`.
    pub set_cursor_position:
        unsafe extern "efiapi" fn(this: *mut Self, column: usize, row: usize) -> Status,
    /// Turns the visibility of the cursor on if `visible` is true, otherwise off.
    pub enable_cursor: unsafe extern "efiapi" fn(this: *mut Self, visible: bool) -> Status,
    /// Pointer to the [`SimpleTextOutputMode`] describing the current state of this [`SimpleTextOutputProtocol`].
    pub mode: *mut SimpleTextOutputMode,
}

impl SimpleTextOutputProtocol {
    /// The [`Guid`] associated with the [`SimpleTextOutputProtocol`].
    pub const GUID: Guid = Guid {
        data1: 0x387477C2,
        data2: 0x69C7,
        data3: 0x11D2,
        data4: [0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B],
    };
}

/// Basic values corresponding to an associated [`SimpleTextOutputProtocol`] that users may utilize.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct SimpleTextOutputMode {
    /// The number of modes supported by [`SimpleTextOutputProtocol::query_mode`] and [`SimpleTextOutputProtocol::set_mode`].
    pub max_mode: i32,

    // Current settings.
    /// The text mode of the device associated with the [`SimpleTextOutputProtocol`].
    pub mode: i32,
    /// The current character output attribute.
    pub attributes: i32,
    /// The cursor's column.
    pub cursor_column: i32,
    /// The cursor's row.
    pub cursor_row: i32,
    /// Whether the cursor is currently visible.
    pub cursor_visible: bool,
}
