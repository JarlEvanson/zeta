//! Wrapper around the UEFI System Table.

use core::{marker::PhantomData, ptr::NonNull};

use uefi::tables::{boot_services::RawBootServicesTable, system::RawSystemTable};

use crate::uefi::{
    protocols::{console::text::SimpleTextOutput, Protocol},
    tables::boot::BootServicesTable,
};

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

    /// Returns the associated [`SimpleTextOutput`] used as console out.
    pub fn console_out(&mut self) -> SystemTableProtocol<SimpleTextOutput> {
        // SAFETY:
        // `self.ptr` points to a valid [`RawSystemTable`].
        let console_out_ptr = unsafe { (*self.ptr.as_ptr()).console_out };
        let console_out =
            // SAFETY:
            // `self.ptr` points to a valid [`RawSystemTable`].
            unsafe { SimpleTextOutput::from_ffi_ptr(console_out_ptr.cast::<core::ffi::c_void>()) };

        SystemTableProtocol {
            protocol: console_out,
            lifetime: PhantomData,
        }
    }

    /// Returns the associated [`SimpleTextOutput`] used as console out.
    pub fn console_err(&mut self) -> SystemTableProtocol<SimpleTextOutput> {
        // SAFETY:
        // `self.ptr` points to a valid [`RawSystemTable`].
        let console_err_ptr = unsafe { (*self.ptr.as_ptr()).console_err };
        let console_err =
            // SAFETY:
            // `self.ptr` points to a valid [`RawSystemTable`].
            unsafe { SimpleTextOutput::from_ffi_ptr(console_err_ptr.cast::<core::ffi::c_void>()) };

        SystemTableProtocol {
            protocol: console_err,
            lifetime: PhantomData,
        }
    }
}

/// [`Protocol`] interfaces offered by a [`SystemTable<Boot>`].
pub struct SystemTableProtocol<'lifetime, T: Protocol> {
    /// The [`Protocol`] struct contains.
    protocol: T,
    /// Maintain's the proper lifetime from the [`SystemTable<Boot>`].
    lifetime: PhantomData<&'lifetime T>,
}

impl<T: Protocol> core::ops::Deref for SystemTableProtocol<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.protocol
    }
}

impl<T: Protocol> core::ops::DerefMut for SystemTableProtocol<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.protocol
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
