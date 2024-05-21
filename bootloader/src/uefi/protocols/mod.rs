//! Wrapper functons and definitions around UEFI protocols.

use uefi::datatypes::Guid;

/// A UEFI protocol.
pub trait Protocol {
    /// The [`Protocol`]'s globally unique ID ([`Guid`]).
    const GUID: Guid;

    /// Create [`Protocol`] from a [`core::ffi::c_void`] pointer.
    ///
    /// # Safety
    /// The input pointer must point to valid data.
    unsafe fn from_ffi_ptr(ptr: *const core::ffi::c_void) -> Self;
}
