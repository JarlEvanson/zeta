//! Wrapper around the UEFI System Table.

use core::{marker::PhantomData, ptr::NonNull};

use uefi::tables::{boot_services::RawBootServicesTable, system::RawSystemTable};

use crate::uefi::tables::boot::BootServicesTable;

/// A UEFI System Table.
pub struct SystemTable<T> {
    /// Pointer to the [`RawSystemTable`].
    ptr: NonNull<RawSystemTable>,
    /// The view of the [`RawSystemTable`].
    view: PhantomData<T>,
}

impl SystemTable<Boot> {
    /// Returns a [`SystemTable`], validating that the `ptr` points to a valid [`RawSystemTable`] as much as possible.
    ///
    /// # Safety
    /// - `ptr` must point to a readable region of memory that is at least `core::mem::size_of::<RawSystemTable>()` bytes.
    pub unsafe fn new(ptr: NonNull<RawSystemTable>) -> Result<SystemTable<Boot>, ()> {
        // SAFETY:
        //
        unsafe { RawSystemTable::validate(ptr.as_ptr()) }.map_err(|_err| ())?;

        // SAFETY:
        // `ptr` points to a readable region of memory that is at least `core::mem::size_of::<RawSystemTable>()` bytes,
        // which is large enough that reading `(*ptr).boot_services` is valid.
        let boot_services_ptr = unsafe { (*ptr.as_ptr()).boot_services };

        // SAFETY:
        //
        unsafe { RawBootServicesTable::validate(boot_services_ptr) }.map_err(|_err| ())?;

        Ok(SystemTable {
            ptr,
            view: PhantomData,
        })
    }

    /// Returns the associated [`BootServicesTable`].
    pub fn boot_services(&mut self) -> BootServicesTable {
        // SAFETY:
        // `self.ptr` points to a valid [`RawSystemTable`].
        let boot_services_ptr = unsafe { (*self.ptr.as_ptr()).boot_services };
        let boot_services_ptr =
            NonNull::new(boot_services_ptr).expect("boot services table unexpectedly changed");

        BootServicesTable {
            ptr: boot_services_ptr,
            lifetime: PhantomData,
        }
    }
}

/// Marker struct associated with the boot view of the UEFI System Table.
pub struct Boot;
impl SystemTableView for Boot {}

/// Marker struct associated with the runtime view of the UEFI System Table.
pub struct Runtime;
impl SystemTableView for Runtime {}

/// A marker trait used to mark different perspectives of the UEFI System Table.
pub trait SystemTableView {}
