//! Wrapper around the UEFI Boot Services Table.

use core::{marker::PhantomData, ptr::NonNull, sync::atomic::AtomicUsize};

use uefi::{datatypes::Status, tables::boot_services::RawBootServicesTable};

/// The number of active [`BootHandle`]s.
static BOOT_HANDLE_COUNT: AtomicUsize = AtomicUsize::new(0);

/// A zero-sized handle to the [`BootServicesTable`].
pub(in crate::uefi) struct BootHandle<'lifetime>(PhantomData<BootServicesTable<'lifetime>>);

/// A UEFI Boot Services Table.
pub struct BootServicesTable<'table> {
    /// Pointer to the [`RawBootServicesTable`].
    pub(in crate::uefi::tables) ptr: NonNull<RawBootServicesTable>,
    /// The lifetime of the [`BootServicesTable`].
    pub(in crate::uefi::tables) lifetime: PhantomData<&'table mut RawBootServicesTable>,
}

impl BootServicesTable<'_> {
    /// Stalls the processor.
    ///
    /// Stalls execution on the processor for at least `microseconds` microseconds.
    /// Execution o fthe processor is not yielded for the duration of the call.
    pub fn stall(&self, microseconds: usize) {
        // SAFETY:
        // `self.ptr` points to a readable [`RawBootServicesTable`].
        let stall_ptr = unsafe { (*self.ptr.as_ptr()).stall };

        // SAFETY:
        // `stall()` was passed valid arguments.
        let result = unsafe { stall_ptr(microseconds) };

        // According to the UEFI specification, `stall()` may only return [`Status::SUCCESS`].
        assert_eq!(result, Status::SUCCESS);
    }
}
