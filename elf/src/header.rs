//! An ELF header contains locations of other important structures in the file, the type
//! of the ELF file, the machine for which the ELF file is meant, and the entry point of
//! the file, if it exists.
//!
//! This module provides parsing functionality for ELF headers.

use core::{
    error::Error,
    fmt::{Debug, Display},
    mem, ptr,
};

use crate::{
    ident::{Class, Ident},
    Elf, ELF_VERSION,
};

/// Contains locations of other important structures in the file, the type of the ELF
/// file, the machine for which the ELF file is meant, and the entry point of the file,
/// if it exists.
pub struct Header<'file> {
    /// The underlying bytes of the header.
    bytes: &'file [u8],
    /// Information about how to parse the ELF header.
    ident: Ident,
}

impl<'file> Header<'file> {
    /// Parses a [`Header`] from bytes, validating its contents.
    ///
    /// # Errors
    /// - [`TooSmall`][ts]
    ///     - Returned if `bytes.len() < 16`.
    /// - [`InvalidVersion`][iv]
    ///     - Returned if the version is invalid.
    ///
    /// [ts]: ParseHeaderError::TooSmall
    /// [iv]: ParseHeaderError::InvalidVersion
    pub fn parse(file: &Elf<'file>) -> Result<Header<'file>, ParseHeaderError> {
        let ident = file.ident;

        let bytes = match ident.class {
            Class::Class32 => file.bytes.get(16..(16 + mem::size_of::<Header32>())),
            Class::Class64 => file.bytes.get(16..(16 + mem::size_of::<Header64>())),
        }
        .ok_or(ParseHeaderError::TooSmall)?;

        let header = Header { bytes, ident };

        if header.version() != u32::from(ELF_VERSION) {
            return Err(ParseHeaderError::InvalidVersion(header.version()));
        }

        Ok(header)
    }

    /// Returns the type of the ELF file.
    #[must_use]
    #[expect(clippy::cast_ptr_alignment, reason = "unaligned reads are used")]
    pub fn object_type(&self) -> ObjectType {
        let ptr = match self.ident.class {
            // SAFETY:
            // During construction, verification that all accesses would be in bounds
            // was done.
            Class::Class32 => unsafe {
                ptr::addr_of!((*self.bytes.as_ptr().cast::<Header32>()).kind)
            },
            // SAFETY:
            // During construction, verification that all accesses would be in bounds
            // was done.
            Class::Class64 => unsafe {
                ptr::addr_of!((*self.bytes.as_ptr().cast::<Header64>()).kind)
            },
        };

        // SAFETY:
        // - `ptr` came from a slice of initialized bytes.
        // - any arrangement of 4 `u8`s make a properly initialized u32.
        let encoded_value = unsafe { ptr.read_unaligned() };
        let decoded_value = self.ident.encoding.decode_u16(encoded_value);

        ObjectType(decoded_value)
    }

    /// Returns the required architecture of the ELF file.
    #[must_use]
    #[expect(clippy::cast_ptr_alignment, reason = "unaligned reads are used")]
    pub fn machine(&self) -> Machine {
        let ptr = match self.ident.class {
            // SAFETY:
            // During construction, verification that all accesses would be in bounds
            // was done.
            Class::Class32 => unsafe {
                ptr::addr_of!((*self.bytes.as_ptr().cast::<Header32>()).machine)
            },
            // SAFETY:
            // During construction, verification that all accesses would be in bounds
            // was done.
            Class::Class64 => unsafe {
                ptr::addr_of!((*self.bytes.as_ptr().cast::<Header64>()).machine)
            },
        };

        // SAFETY:
        // - `ptr` came from a slice of initialized bytes.
        // - any arrangement of 4 `u8`s make a properly initialized u32.
        let encoded_value = unsafe { ptr.read_unaligned() };
        let decoded_value = self.ident.encoding.decode_u16(encoded_value);

        Machine(decoded_value)
    }

    /// Returns the version of the object file. Currently always [`ELF_VERSION`].
    #[must_use]
    #[expect(clippy::cast_ptr_alignment, reason = "unaligned reads are used")]
    pub fn version(&self) -> u32 {
        let ptr = match self.ident.class {
            // SAFETY:
            // During construction, verification that all accesses would be in bounds
            // was done.
            Class::Class32 => unsafe {
                ptr::addr_of!((*self.bytes.as_ptr().cast::<Header32>()).version)
            },
            // SAFETY:
            // During construction, verification that all accesses would be in bounds
            // was done.
            Class::Class64 => unsafe {
                ptr::addr_of!((*self.bytes.as_ptr().cast::<Header64>()).version)
            },
        };

        // SAFETY:
        // - `ptr` came from a slice of initialized bytes.
        // - any arrangement of 4 `u8`s make a properly initialized u32.
        let encoded_value = unsafe { ptr.read_unaligned() };

        self.ident.encoding.decode_u32(encoded_value)
    }

    /// Returns the entry to the object file. If one does not exist, then 0 is returned.
    #[must_use]
    #[expect(clippy::cast_ptr_alignment, reason = "unaligned reads are used")]
    pub fn entry(&self) -> u64 {
        match self.ident.class {
            Class::Class32 => {
                // SAFETY:
                // During construction, verification that all accesses would be in bounds
                // was done.
                let ptr = unsafe { ptr::addr_of!((*self.bytes.as_ptr().cast::<Header32>()).entry) };
                // SAFETY:
                // - `ptr` came from a slice of initialized bytes.
                // - Any arrangment of 4 `u8`s make a properly initialized u32.
                let encoded_value = unsafe { ptr.read_unaligned() };
                u64::from(self.ident.encoding.decode_u32(encoded_value))
            }
            Class::Class64 => {
                // SAFETY:
                // During construction, verification that all accesses would be in bounds
                // was done.
                let ptr = unsafe { ptr::addr_of!((*self.bytes.as_ptr().cast::<Header64>()).entry) };
                // SAFETY:
                // - `ptr` came from a slice of initialized bytes.
                // - Any arrangment of 8 `u8`s make a properly initialized u64.
                let encoded_value = unsafe { ptr.read_unaligned() };
                self.ident.encoding.decode_u64(encoded_value)
            }
        }
    }

    /// Returns the program header table's file offset in bytes. If the program header
    /// does not exist in this file, then 0 is returned.
    #[must_use]
    #[expect(clippy::cast_ptr_alignment, reason = "unaligned reads are used")]
    pub fn ph_offset(&self) -> u64 {
        match self.ident.class {
            Class::Class32 => {
                // SAFETY:
                // During construction, verification that all accesses would be in bounds
                // was done.
                let ptr =
                    unsafe { ptr::addr_of!((*self.bytes.as_ptr().cast::<Header32>()).ph_offset) };
                // SAFETY:
                // - `ptr` came from a slice of initialized bytes.
                // - Any arrangment of 4 `u8`s make a properly initialized u32.
                let encoded_value = unsafe { ptr.read_unaligned() };
                u64::from(self.ident.encoding.decode_u32(encoded_value))
            }
            Class::Class64 => {
                // SAFETY:
                // During construction, verification that all accesses would be in bounds
                // was done.
                let ptr =
                    unsafe { ptr::addr_of!((*self.bytes.as_ptr().cast::<Header64>()).ph_offset) };
                // SAFETY:
                // - `ptr` came from a slice of initialized bytes.
                // - Any arrangment of 8 `u8`s make a properly initialized u64.
                let encoded_value = unsafe { ptr.read_unaligned() };
                self.ident.encoding.decode_u64(encoded_value)
            }
        }
    }

    /// Returns the section header table's file offset in bytes. If the section header
    /// does not exist in this file, then 0 is returned.
    #[must_use]
    #[expect(clippy::cast_ptr_alignment, reason = "unaligned reads are used")]
    pub fn sh_offset(&self) -> u64 {
        match self.ident.class {
            Class::Class32 => {
                // SAFETY:
                // During construction, verification that all accesses would be in bounds
                // was done.
                let ptr =
                    unsafe { ptr::addr_of!((*self.bytes.as_ptr().cast::<Header32>()).sh_offset) };
                // SAFETY:
                // - `ptr` came from a slice of initialized bytes.
                // - Any arrangment of 4 `u8`s make a properly initialized u32.
                let encoded_value = unsafe { ptr.read_unaligned() };
                u64::from(self.ident.encoding.decode_u32(encoded_value))
            }
            Class::Class64 => {
                // SAFETY:
                // During construction, verification that all accesses would be in bounds
                // was done.
                let ptr =
                    unsafe { ptr::addr_of!((*self.bytes.as_ptr().cast::<Header64>()).sh_offset) };
                // SAFETY:
                // - `ptr` came from a slice of initialized bytes.
                // - Any arrangment of 8 `u8`s make a properly initialized u64.
                let encoded_value = unsafe { ptr.read_unaligned() };
                self.ident.encoding.decode_u64(encoded_value)
            }
        }
    }

    /// Returns the processor-specific flags associated with the file.
    #[must_use]
    #[expect(clippy::cast_ptr_alignment, reason = "unaligned reads are used")]
    pub fn flags(&self) -> u32 {
        let ptr = match self.ident.class {
            // SAFETY:
            // During construction, verification that all accesses would be in bounds
            // was done.
            Class::Class32 => unsafe {
                ptr::addr_of!((*self.bytes.as_ptr().cast::<Header32>()).flags)
            },
            // SAFETY:
            // During construction, verification that all accesses would be in bounds
            // was done.
            Class::Class64 => unsafe {
                ptr::addr_of!((*self.bytes.as_ptr().cast::<Header64>()).flags)
            },
        };

        // SAFETY:
        // - `ptr` came from a slice of initialized bytes.
        // - any arrangement of 4 `u8`s make a properly initialized u32.
        let encoded_value = unsafe { ptr.read_unaligned() };

        self.ident.encoding.decode_u32(encoded_value)
    }

    /// Returns the size in bytes of one entry in the file's program header table.
    #[must_use]
    #[expect(clippy::cast_ptr_alignment, reason = "unaligned reads are used")]
    pub fn ph_entry_size(&self) -> u16 {
        let ptr = match self.ident.class {
            // SAFETY:
            // During construction, verification that all accesses would be in bounds
            // was done.
            Class::Class32 => unsafe {
                ptr::addr_of!((*self.bytes.as_ptr().cast::<Header32>()).phent_size)
            },
            // SAFETY:
            // During construction, verification that all accesses would be in bounds
            // was done.
            Class::Class64 => unsafe {
                ptr::addr_of!((*self.bytes.as_ptr().cast::<Header64>()).phent_size)
            },
        };

        // SAFETY:
        // - `ptr` came from a slice of initialized bytes.
        // - any arrangement of 4 `u8`s make a properly initialized u32.
        let encoded_value = unsafe { ptr.read_unaligned() };

        self.ident.encoding.decode_u16(encoded_value)
    }

    /// Returns the number of entries in the file's program header table.
    #[must_use]
    #[expect(clippy::cast_ptr_alignment, reason = "unaligned reads are used")]
    pub fn ph_entry_count(&self) -> u16 {
        let ptr = match self.ident.class {
            // SAFETY:
            // During construction, verification that all accesses would be in bounds
            // was done.
            Class::Class32 => unsafe {
                ptr::addr_of!((*self.bytes.as_ptr().cast::<Header32>()).phent_num)
            },
            // SAFETY:
            // During construction, verification that all accesses would be in bounds
            // was done.
            Class::Class64 => unsafe {
                ptr::addr_of!((*self.bytes.as_ptr().cast::<Header64>()).phent_num)
            },
        };

        // SAFETY:
        // - `ptr` came from a slice of initialized bytes.
        // - any arrangement of 4 `u8`s make a properly initialized u32.
        let encoded_value = unsafe { ptr.read_unaligned() };

        self.ident.encoding.decode_u16(encoded_value)
    }

    /// Returns the size in bytes of one entry in the file's section header table.
    #[must_use]
    #[expect(clippy::cast_ptr_alignment, reason = "unaligned reads are used")]
    pub fn sh_entry_size(&self) -> u16 {
        let ptr = match self.ident.class {
            // SAFETY:
            // During construction, verification that all accesses would be in bounds
            // was done.
            Class::Class32 => unsafe {
                ptr::addr_of!((*self.bytes.as_ptr().cast::<Header32>()).shent_size)
            },
            // SAFETY:
            // During construction, verification that all accesses would be in bounds
            // was done.
            Class::Class64 => unsafe {
                ptr::addr_of!((*self.bytes.as_ptr().cast::<Header64>()).shent_size)
            },
        };

        // SAFETY:
        // - `ptr` came from a slice of initialized bytes.
        // - any arrangement of 4 `u8`s make a properly initialized u32.
        let encoded_value = unsafe { ptr.read_unaligned() };

        self.ident.encoding.decode_u16(encoded_value)
    }

    /// Returns the number of entries in the file's section header table.
    #[must_use]
    #[expect(clippy::cast_ptr_alignment, reason = "unaligned reads are used")]
    pub fn sh_entry_count(&self) -> u16 {
        let ptr = match self.ident.class {
            // SAFETY:
            // During construction, verification that all accesses would be in bounds
            // was done.
            Class::Class32 => unsafe {
                ptr::addr_of!((*self.bytes.as_ptr().cast::<Header32>()).shent_num)
            },
            // SAFETY:
            // During construction, verification that all accesses would be in bounds
            // was done.
            Class::Class64 => unsafe {
                ptr::addr_of!((*self.bytes.as_ptr().cast::<Header64>()).shent_num)
            },
        };

        // SAFETY:
        // - `ptr` came from a slice of initialized bytes.
        // - any arrangement of 4 `u8`s make a properly initialized u32.
        let encoded_value = unsafe { ptr.read_unaligned() };

        self.ident.encoding.decode_u16(encoded_value)
    }

    /// Returns the index of the entry in the section header table that is associated
    /// with the section name string table.
    #[must_use]
    #[expect(clippy::cast_ptr_alignment, reason = "unaligned reads are used")]
    pub fn shstrndx(&self) -> u16 {
        let ptr = match self.ident.class {
            // SAFETY:
            // During construction, verification that all accesses would be in bounds
            // was done.
            Class::Class32 => unsafe {
                ptr::addr_of!((*self.bytes.as_ptr().cast::<Header32>()).shstrndx)
            },
            // SAFETY:
            // During construction, verification that all accesses would be in bounds
            // was done.
            Class::Class64 => unsafe {
                ptr::addr_of!((*self.bytes.as_ptr().cast::<Header64>()).shstrndx)
            },
        };

        // SAFETY:
        // - `ptr` came from a slice of initialized bytes.
        // - any arrangement of 4 `u8`s make a properly initialized u32.
        let encoded_value = unsafe { ptr.read_unaligned() };

        self.ident.encoding.decode_u16(encoded_value)
    }
}

impl Debug for Header<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut dstruct = f.debug_struct("Header");

        dstruct.field("kind", &self.object_type());
        dstruct.field("machine", &self.machine());
        dstruct.field("version", &self.version());
        dstruct.field("entry", &self.entry());
        dstruct.field("ph_offset", &self.ph_offset());
        dstruct.field("sh_offset", &self.sh_offset());
        dstruct.field("flags", &self.flags());
        dstruct.field("ph_entry_size", &self.ph_entry_size());
        dstruct.field("ph_entry_count", &self.ph_entry_count());
        dstruct.field("ph_entry_size", &self.ph_entry_size());
        dstruct.field("ph_entry_count", &self.ph_entry_count());
        dstruct.field("shshtndx", &self.shstrndx());

        dstruct.finish()
    }
}

/// Various errors that can occur while parsing a [`Header`].
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParseHeaderError {
    /// The ELF file was too small.
    TooSmall,
    /// The version is not [`ELF_VERSION`].
    InvalidVersion(u32),
}

impl Display for ParseHeaderError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ParseHeaderError::TooSmall => write!(f, "file was too small to contain an ELF header"),
            ParseHeaderError::InvalidVersion(version) => {
                write!(f, "invalid ELF version: {version}, expect {ELF_VERSION}")
            }
        }
    }
}

impl Error for ParseHeaderError {}

/// Identifies the object file type.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObjectType(pub u16);

impl ObjectType {
    /// No file type.
    pub const NONE: ObjectType = ObjectType(0);
    /// The ELF file is relocatable.
    pub const RELOCATABLE: ObjectType = ObjectType(1);
    /// The ELF file is executable.
    pub const EXECUTABLE: ObjectType = ObjectType(2);
    /// The ELF file is a shared object.
    pub const SHARED_OBJECT: ObjectType = ObjectType(3);
    /// The ELF file is a core file.
    pub const CORE: ObjectType = ObjectType(4);
    /// Inclusive start of the operating system-specific range.
    pub const OS_SPECIFIC_START: ObjectType = ObjectType(0xFE00);
    /// Inclusive end of the operating system-specific range.
    pub const OS_SPECIFIC_END: ObjectType = ObjectType(0xFEFF);
    /// Inclusive start of the processor-specific range.
    pub const PROCESSOR_SPECIFIC_START: ObjectType = ObjectType(0xFF00);
    /// Inclusive end of the processor-specific range.
    pub const PROCESSOR_SPECIFIC_END: ObjectType = ObjectType(0xFFFF);

    /// Tests whether the kind of this file is operating system-specific.
    #[must_use]
    pub fn is_os_specific(self) -> bool {
        Self::OS_SPECIFIC_START <= self && self <= Self::OS_SPECIFIC_END
    }

    /// Tests whether the kind of this file is processor-specific.
    #[must_use]
    pub fn is_processor_specific(self) -> bool {
        Self::PROCESSOR_SPECIFIC_START <= self && self <= Self::PROCESSOR_SPECIFIC_END
    }
}

/// Identifies the required architecture of the ELF file.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Machine(pub u16);

impl Machine {
    /// No machine
    pub const NONE: Machine = Machine(0);
    /// AMD x86-64 architecture
    pub const X86_64: Machine = Machine(62);
}

/// The underlying structure of the [`Header`] on 32-bit architectures.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct Header32 {
    /// Identifies the object file type.
    kind: u16,
    /// Specifies the required architecture for the ELF file.
    machine: u16,
    /// Identifies the current object file version.
    version: u32,
    /// The virtual address to which the system first transfers control.
    /// If the file has no entry point, this is 0
    entry: u32,
    /// The offset to the start of the program header table in bytes.
    ph_offset: u32,
    /// The offset to the start of the section header table in bytes.
    sh_offset: u32,
    /// Processor specific flags associated with the file.
    flags: u32,
    /// The ELF header's size in bytes.
    eh_size: u16,
    /// The size in bytes of one entry in the file's program header table.
    phent_size: u16,
    /// The number of entries in the file's program header table.
    phent_num: u16,
    /// The size in bytes of one entry in the file's section header table.
    shent_size: u16,
    /// The number of entries in the file's program header table.
    shent_num: u16,
    /// The index into the section header table that is associated with the section name string table.
    shstrndx: u16,
}

/// The underlying structure of the [`Header`] on 64-bit architectures.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct Header64 {
    /// Identifies the object file type.
    kind: u16,
    /// Specifies the required architecture for the ELF file.
    machine: u16,
    /// Identifies the current object file version.
    version: u32,
    /// The virtual address to which the system first transfers control.
    /// If the file has no entry point, this is 0
    entry: u64,
    /// The offset to the start of the program header table in bytes.
    ph_offset: u64,
    /// The offset to the start of the section header table in bytes.
    sh_offset: u64,
    /// Processor specific flags associated with the file.
    flags: u32,
    /// The ELF header's size in bytes.
    eh_size: u16,
    /// The size in bytes of one entry in the file's program header table.
    phent_size: u16,
    /// The number of entries in the file's program header table.
    phent_num: u16,
    /// The size in bytes of one entry in the file's section header table.
    shent_size: u16,
    /// The number of entries in the file's program header table.
    shent_num: u16,
    /// The index into the section header table that is associated with the section name
    /// string table.
    shstrndx: u16,
}
