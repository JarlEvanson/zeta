//! The API for Zeta project bootloader.

#![no_std]

/// The type of a physical memory range.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum PhysicalMemoryType {
    /// Free usable memory.
    Conventional,
    /// Memory occupied by the kernel code and static data.
    Kernel,
    /// Memory occupied by UEFI runtime services.
    Runtime,
    /// Memory that contains the initial data passed from the bootloader.
    InitTables,
    /// Physical memory range containing the framebuffer.
    Framebuffer,
    /// Memory that holds ACPI tables.
    ///
    /// Can be reclaimed after they are parsed.
    AcpiReclaimable,
    /// Firmware-reserved addresses.
    AcpiNonVolatile,
    /// A region used for memory-mapped I/O.
    Mmio,
    /// A region used for memory-mapped port I/O.
    MmioPortSpace,
    /// Custom memory types.
    Custom(u32),
}
