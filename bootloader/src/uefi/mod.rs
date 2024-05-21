//! UEFI wrapper for the bootloader.

use core::{
    ptr,
    sync::atomic::{AtomicPtr, Ordering},
};

pub use uefi::{
    datatypes::{Handle, RawHandle, Status},
    tables::system::RawSystemTable,
};

pub mod protocols;
pub mod tables;

/// The private access to the system table used for destructors.
static SYSTEM_TABLE: AtomicPtr<RawSystemTable> = AtomicPtr::new(ptr::null_mut());

/// The executable's handle.
static IMAGE_HANDLE: AtomicPtr<core::ffi::c_void> = AtomicPtr::new(ptr::null_mut());

/// Sets [`SYSTEM_TABLE`] to `ptr`.
///
/// # Safety
/// `ptr` must point to the executable's [`RawSystemTable`].
pub unsafe fn set_system_table(ptr: *mut RawSystemTable) {
    SYSTEM_TABLE.store(ptr, Ordering::Relaxed);
}

/// Sets [`IMAGE_HANDLE`] to `handle`.
///
/// # Safety
/// `handle` must be the executable's handle.
pub unsafe fn set_image_handle(handle: RawHandle) {
    IMAGE_HANDLE.store(handle.0, Ordering::Relaxed)
}

/// Defines the entry point function.
#[macro_export]
macro_rules! entry_point {
    ($path:path) => {
        #[doc(hidden)]
        #[export_name = "efi_main"]
        pub unsafe extern "efiapi" fn __efi_main(
            raw_handle: $crate::uefi::RawHandle,
            system_table: *mut $crate::uefi::RawSystemTable,
        ) -> $crate::uefi::Status {
            const ENTRY_POINT: fn($crate::uefi::Handle, $crate::uefi::tables::system::SystemTable<$crate::uefi::tables::system::Boot>) -> $crate::uefi::Status = $path;

            let Some(system_table_ptr) = core::ptr::NonNull::new(system_table) else {
                return $crate::uefi::Status::INVALID_PARAMETER;
            };

            let system_table =
                // SAFETY:
                // `system_table_ptr` has been validated as much as possible.
                match unsafe { $crate::uefi::tables::system::SystemTable::new(system_table_ptr) } {
                    Ok(table) => table,
                    Err(error) => match error {
                        _ => return $crate::uefi::Status::INVALID_PARAMETER,
                    },
                };

            let Some(handle) = $crate::uefi::Handle::new(raw_handle) else {
                return $crate::uefi::Status::INVALID_PARAMETER;
            };

            // SAFETY:
            // `system_table` has been validated as much as it can be,
            // and was provided to the executable as the executable's [`RawSystemTable`].
            unsafe { $crate::uefi::set_system_table(system_table_ptr.as_ptr()) };

            // SAFETY:
            // `raw_handle` was provided to `efi_main` as the executable's handle.
            unsafe { $crate::uefi::set_image_handle(raw_handle) };

            ENTRY_POINT(handle, system_table)
        }
    };
}
