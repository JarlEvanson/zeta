//! Support for booting using the Limine boot protocol.

use core::marker::PhantomData;

use crate::{
    cells::ControllledModificationCell,
    log, log_debug, log_trace,
    logging::{self, LogFilter, LogLevel},
    utils::u64_to_usize,
};

/// The base revision of the Limine boot protocol that this kernel expects to be booted from.
const BASE_REVISION: u64 = 2;

/// The base revision tag that allows the bootloader to be able to identify the tag and specifies the
/// [`BASE_REVISION`] that this kernel requires.
#[used]
#[link_section = ".limine.requests"]
static BASE_REVISION_TAG: ControllledModificationCell<[u64; 3]> =
    ControllledModificationCell::new([0xf9562b2d5c95a6c8, 0x6a7b384944536bdc, BASE_REVISION]);

/// The request that specifies the entry point that the Limine bootloader should use to boot the kernel.
#[used]
#[link_section = ".limine.requests"]
static ENTRY_POINT_REQUEST: ControllledModificationCell<EntryPointRequest> =
    ControllledModificationCell::new(EntryPointRequest {
        header: RequestHeader::new(),
        entry: Some(entry),
    });

/// The kernel requires knowing the layout of physical memory.
#[used]
#[link_section = ".limine.requests"]
static MEMORY_MAP_REQUEST: ControllledModificationCell<MemoryMapRequest> =
    ControllledModificationCell::new(MemoryMapRequest {
        header: RequestHeader::new(),
    });

/// Entry point for the Limine bootloader.
///
/// # Panics
/// If the Limine bootloader utilizes old revisions of the Limine boot protocol.
#[optimize(size)]
#[export_name = "LIMINE_ENTRY"]
#[link_section = ".limine.entry"]
pub extern "C" fn entry() -> ! {
    // SAFETY:
    // Currently, no other ports are used.
    unsafe { logging::set_logger(crate::arch::x86_64::DebugConLogger::new_mut()) }
    logging::set_filter(LogFilter::Trace);

    log!(LogLevel::Info, "Booted from Limine boot protocol");

    // Validate fundamental requirements.
    assert!(BASE_REVISION_TAG.get()[2] == 0);
    log_trace!("Validated base revision tag");
    assert!(ENTRY_POINT_REQUEST.get().header.processed_as_provided());
    log_trace!("Validated entry point response");

    // Start parsing and validating the memory map.
    assert!(MEMORY_MAP_REQUEST.get().header.processed_as_provided());
    log_trace!("Validated that the memory map was processed correctly");
    let memory_map_response = MEMORY_MAP_REQUEST.get().header.response();
    log_trace!("Acquired memory map response");
    assert!(memory_map_response.header.expected_revision());
    log_trace!("Validated understandable revision");

    let entries = memory_map_response.entries();

    for &entry in entries.iter() {
        log_debug!("Entry: {:#X} {:X} {}", entry.base, entry.length, entry.kind);
        core::hint::black_box(entry);
    }

    log_trace!("Entering sleep loop");

    loop {
        // SAFETY:
        // halting when we have nothing to do is safe.
        unsafe { core::arch::asm!("hlt") }
    }
}

/// The header of all Limine boot protocol requests.
#[repr(C)]
struct RequestHeader<T: Request> {
    /// The ID of the request. There may only be one of the same request.
    id: [u64; 4],
    /// The revision of the request that that kernel requires. Bootloaders process requests in a backwards
    /// compatible manner always, which means that if the booloader does not support the revision of the request,
    /// it will process the request as if it were the highest revision that the bootloader supports.
    revision: u64,
    /// This field is filled in by the bootloader at load time, with a pointer to the response structure, if the
    /// request was sucessfully processed. If the request is unsupported or was not successfully processed, this field
    /// is left untouched.
    response: *mut T::Response,
}

// SAFETY:
// All API's provided by [`RequestHeader`] are safe.
unsafe impl<T: Request> Sync for RequestHeader<T> {}

impl<T: Request> RequestHeader<T> {
    /// The first 64-bit integer that begins every Limine request id.
    const COMMON_MAGIC_1: u64 = 0xc7b1dd30df4c8b88;

    /// The second 64-bit integer that begins every Limine request id.
    const COMMON_MAGIC_2: u64 = 0x0a82e883a194f07b;

    /// Creates a new [`RequestHeader`] ready to be placed in the binary.
    pub const fn new() -> RequestHeader<T> {
        RequestHeader {
            id: [
                RequestHeader::<T>::COMMON_MAGIC_1,
                RequestHeader::<T>::COMMON_MAGIC_2,
                T::MAGIC_3,
                T::MAGIC_4,
            ],
            revision: T::REVISION,
            response: core::ptr::null_mut(),
        }
    }

    /// Returns the revision of the protocol that the request was processed as.
    pub const fn processed_as(&self) -> u64 {
        self.revision
    }

    /// Returns whether the bootloader supports the provided revision of the [`Request`].
    pub const fn processed_as_provided(&self) -> bool {
        self.revision >= T::REVISION
    }

    /// Returns the [`T::Response`] that to which this [`RequestHeader`] points.
    pub fn response(&self) -> &T::Response {
        // SAFETY:
        unsafe { self.response.as_mut().unwrap() }
    }
}

/// Limine requests.
trait Request {
    /// The third 64-bit integer that begins the Limine request.
    const MAGIC_3: u64;
    /// The last 64-bit integer that begins the Limine request.
    const MAGIC_4: u64;

    /// The revision of the request that the kernel provides. Bootloaders process requests in a backwards
    /// compatible manner always, which means that if the booloader does not support the revision of the request,
    /// it will process the request as if it were the highest revision that the bootloader supports.
    const REVISION: u64;

    /// The type of the response associated with the [`Request`].
    type Response: Response;
}

/// The header of all Limine boot protocol responses.
#[repr(C)]
struct ResponseHeader<T: Response> {
    /// The revision of the response that the bootloader provides.
    ///
    /// This is always backwards compatible, which means that higher revisions support all that lower revisions do.
    revision: u64,
    /// Phantom data.
    phantom: PhantomData<T>,
}

impl<T: Response> ResponseHeader<T> {
    /// Returns `true` if the revision of the [`Response`] that the bootloader provided
    /// is compatible with the version this kernel expects.
    pub fn expected_revision(&self) -> bool {
        self.revision >= T::REVISION
    }

    /// Returns the revision of the [`Response`] that the bootloader provided.
    pub fn revision(&self) -> u64 {
        self.revision
    }
}

/// Limine responses.
trait Response {
    /// The revision of the response that the bootloader provides.
    ///
    /// This is always backwards compatible, which means that higher revisions support all that lower revisions do.
    const REVISION: u64;
}

/// Specifies the entry point that the Limine bootloader should use to boot this kernel.
#[repr(C)]
struct EntryPointRequest {
    /// The header for [`EntryPointRequest`].
    header: RequestHeader<EntryPointRequest>,
    /// The entry point that the limine bootloader should use.
    entry: Option<extern "C" fn() -> !>,
}

impl Request for EntryPointRequest {
    const MAGIC_3: u64 = 0x13d86c035a1cd3e1;
    const MAGIC_4: u64 = 0x2b0caa89d8f3026a;
    const REVISION: u64 = 0;
    type Response = EntryPointResponse;
}

/// The response to an [`EntryPointRequest`].
#[repr(C)]
struct EntryPointResponse {
    /// The header for [`EntryPointResponse`].
    header: ResponseHeader<EntryPointResponse>,
}

impl Response for EntryPointResponse {
    const REVISION: u64 = 0;
}

/// Requests the physical memory map for the system.
#[repr(C)]
struct MemoryMapRequest {
    /// The header for [`MemoryMapRequest`].
    header: RequestHeader<MemoryMapRequest>,
}

// SAFETY:
// All API's provided by [`MemoryMapRequest`] are safe.
unsafe impl Sync for MemoryMapRequest {}

impl Request for MemoryMapRequest {
    const MAGIC_3: u64 = 0x67cf3d9d378a806f;
    const MAGIC_4: u64 = 0xe304acdfc50c3c62;
    const REVISION: u64 = 0;
    type Response = MemoryMapResponse;
}

/// The response to a [`MemoryMapRequest`].
#[repr(C)]
struct MemoryMapResponse {
    /// The header for [`MemoryMapResponse`].
    header: ResponseHeader<MemoryMapResponse>,
    /// The number of [`MemoryMapEntry`] structures returned.
    entry_count: u64,
    /// Pointer to an array of `entry_count` pointers to [`MemoryMapEntry`] structures.
    entries: *mut *mut MemoryMapEntry,
}

impl MemoryMapResponse {
    /// Returns all the entries in the [`MemoryMapResponse`].
    pub fn entries(&self) -> &[&MemoryMapEntry] {
        // SAFETY:
        // According to the Limine protocol, this should be safe.
        unsafe {
            core::slice::from_raw_parts(
                self.entries.cast::<&MemoryMapEntry>(),
                u64_to_usize(self.entry_count),
            )
        }
    }
}

impl Response for MemoryMapResponse {
    const REVISION: u64 = 0;
}

/// Structure describing the layout of a single entry in the Limine memory map.
#[repr(C)]
struct MemoryMapEntry {
    /// The base address of the physical memory region described by the [`MemoryMapEntry`].
    base: u64,
    /// The size, in bytes, of the physical memory region described by the [`MemoryMapEntry`].
    length: u64,
    /// The kind of the physical memory region described by the [`MemoryMapEntry`].
    kind: u64,
}
