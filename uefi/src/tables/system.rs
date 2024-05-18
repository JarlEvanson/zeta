//! Definitions and interfaces for interacting with the UEFI system table.

use crate::{
    datatypes::{Char16, RawHandle},
    protocols::console::text::SimpleTextOutputProtocol,
    tables::{Header, TableHeaderValidationError},
};

/// Container for both the runtime and boot services tables, as well as configuration tables
/// and the standard input, output, and error protocols and associated [`Handle`]'s.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct RawSystemTable {
    /// The [`Header`] used to check validity of this [`RawSystemTable`].
    pub header: Header,

    /// A pointer to a null terminated string that identifies the vendor that produces the system firmware
    /// for the platform.
    pub firmware_vendor: *const Char16,
    /// A firmware vendor specific value that identifies the revision of the system firmware for the platform.
    pub firmware_revision: u32,

    /// The handle for the active console input device.
    pub console_in_handle: RawHandle,
    /// A pointer to the protocol associated with [`RawSystemTable::console_in_handle`].
    pub console_in: *mut (),

    /// The handle for the active console output device.
    pub console_out_handle: RawHandle,
    /// A pointer to the protocol associated with [`RawSystemTable::console_out_handle`].
    pub console_out: *mut (),

    /// The handle for the active console error device.
    pub console_err_handle: RawHandle,
    /// A pointer to the protocol associated with [`RawSystemTable::console_err_handle`].
    pub console_err: *mut (),

    /// A pointer to the UEFI runtime services table.
    pub runtime_services: *mut (),
    /// A pointer to the UEFI boot services table.
    pub boot_services: *mut (),

    /// The number of system configuration tables in the buffer pointed to
    /// by [`RawSystemTable::configuration_tables`].
    pub table_entry_count: usize,
    /// A pointer to the system configuration tables.
    pub configuration_tables: *mut (),
}

impl RawSystemTable {
    /// The 64-bit signature that identifies the table as a UEFI [`RawSystemTable`].
    pub const SIGNATURE: u64 = 0x5453_5953_2049_4249;

    /// Validates that the provided pointer points to a valid UEFI [`RawSystemTable`].
    ///
    /// # Safety
    /// - `ptr` must be valid for reads.
    /// - `ptr` must point to a region of memory that is properly-aligned and at least `core::mem::size_of<Header>()` bytes
    ///     or `ptr.size` bytes, whichever is larger.
    /// - The region of memory to which `ptr` points must be properly initialized up to the required number of bytes.
    ///
    /// # Errors
    /// - [`InvalidSignature`][is]
    ///     - The signature of the [`RawSystemTable`] is not [`RawSystemTable::SIGNATURE`].
    /// - [`NonZeroReserved`][nzr]
    ///     - The reserved field of [`RawSystemTable`] is non-zero.
    /// - [`InvalidCrc32`][ic32]
    ///     - The expected 32-bit CRC of the [`RawSystemTable`] did not equal the calculated 32-bit CRC.
    ///
    /// [is]: TableHeaderValidationError::InvalidSignature
    /// [nzr]: TableHeaderValidationError::NonZeroReserved
    /// [ic32]: TableHeaderValidationError::InvalidCrc32
    pub unsafe fn validate(ptr: *const RawSystemTable) -> Result<(), TableHeaderValidationError> {
        // SAFETY:
        // The invariants of `Header::validate()` are the same as this function's invariants.
        unsafe { Header::validate(RawSystemTable::SIGNATURE, ptr.cast::<Header>()) }
    }
}
