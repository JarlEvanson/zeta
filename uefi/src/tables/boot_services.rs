//! Definitions and interfaces for interacting with the UEFI boot services table.

use crate::{
    datatypes::Status,
    tables::{Header, TableHeaderValidationError},
};

/// A container for function pointers to interact with the UEFI environment.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct RawBootServicesTable {
    /// The [`Header`] used to check validity of this [`RawBootServicesTable`].
    pub header: Header,

    /// Raises the task priority level.
    pub raise_tpl: unsafe extern "efiapi" fn(),
    /// Restores/lowers the task priority level.
    pub restore_tpl: unsafe extern "efiapi" fn(),

    /// Allocates pages of a particular type.
    pub allocate_pages: unsafe extern "efiapi" fn(),
    /// Frees allocated pages.
    pub free_pages: unsafe extern "efiapi" fn(),
    /// Returns the current boot services memory map and memory map key.
    pub get_memory_map: unsafe extern "efiapi" fn(),
    /// Allocates a pool of a particular type.
    pub allocate_pool: unsafe extern "efiapi" fn(),
    /// Frees an allocated pool.
    pub free_pool: unsafe extern "efiapi" fn(),

    /// Creates a general purpose event structure.
    pub create_event: unsafe extern "efiapi" fn(),
    /// Sets an event to be signaled at a particular time.
    pub set_timer: unsafe extern "efiapi" fn(),
    /// Stops execution until an event is signaled.
    pub wait_for_event: unsafe extern "efiapi" fn(),
    /// Signals an event.
    pub signal_event: unsafe extern "efiapi" fn(),
    /// Closes and frees an event structure.
    pub close_event: unsafe extern "efiapi" fn(),
    /// Checks whether an event is in the signaled state.
    pub check_event: unsafe extern "efiapi" fn(),

    /// Installs a protocol interface on a device handle.
    pub install_protocol_interface: unsafe extern "efiapi" fn(),
    /// Reinstalls a protocol interface on a device handle.
    pub reinstall_protocol_interface: unsafe extern "efiapi" fn(),
    /// Removes a protocol interface from a device handle.
    pub uninstall_protocol_interface: unsafe extern "efiapi" fn(),
    /// Queries a handle to determine if it supports a specified protocol.
    pub handle_protocol: unsafe extern "efiapi" fn(),
    /// Must be null.
    pub _reserved: *mut (),
    /// Registers an event that is to be signaled whenever an interface is installed for a
    /// specified protocol.
    pub register_protocol_notify: unsafe extern "efiapi" fn(),
    /// Returns an array of handles that support a specified protocol.
    pub locate_handle: unsafe extern "efiapi" fn(),
    /// Locates all devices on a device path that support a specified protocol and returns
    /// the handle ot the device that is closest to the path.
    pub locate_device_path: unsafe extern "efiapi" fn(),
    /// Adds, updates, or removes a configuration table from the UEFI [`RawSystemTable`][rst].
    ///
    /// [rst]: crate::tables::system::RawSystemTable
    pub install_configuration_table: unsafe extern "efiapi" fn(),

    /// Loads an EFI image into memory.
    pub load_image: unsafe extern "efiapi" fn(),
    /// Transfers control to a loaded image's entry point.
    pub start_image: unsafe extern "efiapi" fn(),
    /// Exits the image's entry point.
    pub exit: unsafe extern "efiapi" fn(),
    /// Unloads an image.
    pub unload_image: unsafe extern "efiapi" fn(),
    /// Terminates boot services.
    pub exit_boot_services: unsafe extern "efiapi" fn(),

    /// Returns a monotonically increasing count for the platform.
    pub get_next_monotonic_count: unsafe extern "efiapi" fn(),
    /// Stalls the processor.
    pub stall: unsafe extern "efiapi" fn(microseconds: usize) -> Status,
    /// Resets and sets the watchdog timer used during boot services.
    pub set_watchdog_timer: unsafe extern "efiapi" fn(),

    /// Uses a set of precedence rules to find the best set of drivers to manage
    /// a controller.
    pub connect_controller: unsafe extern "efiapi" fn(),
    /// Informs a set of drivers to stop managing a controller.
    pub disconnect_controller: unsafe extern "efiapi" fn(),

    /// Adds elements to a list of agents consuming a protocol interface.
    pub open_protocol: unsafe extern "efiapi" fn(),
    /// Removes elements from the list of agents consuming a protocol interface.
    pub close_protocol: unsafe extern "efiapi" fn(),
    /// Retrieves the list of agents that are currently consuming a protocol interface.
    pub open_protocol_information: unsafe extern "efiapi" fn(),

    /// Retrieves the list of protocols installed on a handle. The return buffer is
    /// allocated automatically.
    pub protocols_per_handle: unsafe extern "efiapi" fn(),
    /// Retrieves the list of handles from the handle database that meet the search
    /// criteria. The return buffer is allocated automatically.
    pub locate_handle_buffer: unsafe extern "efiapi" fn(),
    /// Finds the first handle in the handle database that supports the requested protocol.
    pub locate_protocol: unsafe extern "efiapi" fn(),
    /// Installs one or more protocol interfaces onto a handle.
    pub install_multiple_protocol_interface: unsafe extern "efiapi" fn(),
    /// Removes one or more protocol interfaces from a handle.
    pub uninstall_multiple_protocol_interface: unsafe extern "efiapi" fn(),

    /// Computes and returns a 32-bit CRC for a data buffer.
    pub calculate_crc32: unsafe extern "efiapi" fn(),

    /// Copies the contents of one buffer to another buffer.
    pub copy_mem: unsafe extern "efiapi" fn(),
    /// Fills a buffer with a specified value.
    pub set_mem: unsafe extern "efiapi" fn(),
    /// Creates an event structure as part of an event group.
    pub create_event_ex: unsafe extern "efiapi" fn(),
}

impl RawBootServicesTable {
    /// The 64-bit signature that identifies the table as a UEFI [`RawBootServicesTable`].
    pub const SIGNATURE: u64 = 0x5652_4553_544f_4f42;

    /// Validates that the provided pointer points to a valid UEFI [`RawBootServicesTable`].
    ///
    /// # Safety
    /// - `ptr` must be valid for reads.
    /// - `ptr` must point to a region of memory that is properly-aligned and at least `core::mem::size_of<Header>()` bytes
    ///     or `ptr.size` bytes, whichever is larger.
    /// - The region of memory to which `ptr` points must be properly initialized up to the required number of bytes.
    ///
    /// # Errors
    /// - [`InvalidSignature`][is]
    ///     - The signature of the [`RawBootServicesTable`] is not [`RawBootServicesTable::SIGNATURE`].
    /// - [`NonZeroReserved`][nzr]
    ///     - The reserved field of [`RawBootServicesTable`] is non-zero.
    /// - [`InvalidCrc32`][ic32]
    ///     - The expected 32-bit CRC of the [`RawBootServicesTable`] did not equal the calculated 32-bit CRC.
    ///
    /// [is]: TableHeaderValidationError::InvalidSignature
    /// [nzr]: TableHeaderValidationError::NonZeroReserved
    /// [ic32]: TableHeaderValidationError::InvalidCrc32
    /// - `ptr` points a region that is at least `(*ptr).size` bytes.
    pub unsafe fn validate(
        ptr: *mut RawBootServicesTable,
    ) -> Result<(), TableHeaderValidationError> {
        // SAFETY:
        // The invariants of `Header::validate()` are the same as this function's invariants.
        unsafe { Header::validate(RawBootServicesTable::SIGNATURE, ptr.cast::<Header>()) }
    }
}
