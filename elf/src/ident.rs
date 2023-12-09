//! The initial bytes mark the file as an object file and provide machine-independent
//! data with which to decode and interpret the file's contents.
//!
//! This module provides parsing functionality for the referenced data.

use core::{fmt::Display, error::Error};

use crate::ELF_VERSION;

/// All the machine independent data needed to successfully parse an ELF file.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ident {
    /// The class of the FILE file.
    pub class: Class,
    /// The encoding of the data in the FILE file.
    pub encoding: Encoding,
    /// The OS- or ABI-specific ELF extensions used by the ELF file.
    pub os_abi: OsAbi,
    /// The version of the ABI to which the object is targeted.
    pub os_abi_version: u8,
}

impl Ident {
    /// The magic bytes
    const MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];

    /// Parses a [`Ident`] from bytes, validating its contents.
    ///
    /// # Errors
    /// - [`TooSmall`][ts]
    ///     - Returned if `bytes.len() < 16`.
    /// - [`IncorrectMagic`][im]
    ///     - Returned if the magic value is incorrect.
    /// - [`InvalidClass`][ic]
    ///     - Returned if the class is not 1 or 2 ([`Class32`][c32] or [`Class64`][c64]) 
    /// - [`InvalidEncoding`][ie]
    ///     - Returned if the encoding is not 1 or 2 ([`LittleEndian`][le] or [`BigEndian`][be])
    /// - [`InvalidVersion`][iv]
    ///     - Returned if the version is invalid.
    /// 
    /// [ts]: ParseIdentError::TooSmall
    /// [im]: ParseIdentError::IncorrectMagic
    /// [ic]: ParseIdentError::InvalidClass
    /// [c32]: Class::Class32
    /// [c64]: Class::Class64
    /// [ie]: ParseIdentError::InvalidEncoding
    /// [le]: Encoding::LittleEndian
    /// [be]: Encoding::BigEndian
    /// [iv]: ParseIdentError::InvalidVersion
    pub fn parse(bytes: &[u8]) -> Result<Ident, ParseIdentError> {
        if bytes.len() < 16 {
            return Err(ParseIdentError::TooSmall);
        }

        if bytes[0..4] != Self::MAGIC {
            // SAFETY:
            // `bytes[0..4` is a slice of exactly 4 bytes.
            let buffer = unsafe { TryInto::<[u8; 4]>::try_into(&bytes[0..4]).unwrap_unchecked() };
            return Err(ParseIdentError::IncorrectMagic(buffer));
        }

        let class = match bytes[4] {
            1 => Class::Class32,
            2 => Class::Class64,
            byte => return Err(ParseIdentError::InvalidClass(byte)),
        };

        let encoding = match bytes[5] {
            1 => Encoding::LittleEndian,
            2 => Encoding::BigEndian,
            byte => return Err(ParseIdentError::InvalidEncoding(byte)),
        };

        if bytes[6] != ELF_VERSION {
            return Err(ParseIdentError::InvalidVersion(bytes[6]));
        }

        let os_abi = OsAbi(bytes[7]);
        let os_abi_version = bytes[8];

        let ident = Ident {
            class,
            encoding,
            os_abi,
            os_abi_version,
        };

        Ok(ident)
    }
}

/// Various errors that can occur while parsing an [`Ident`].
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParseIdentError {
    /// The magic bytes were incorrect.
    IncorrectMagic([u8; 4]),
    /// The [`Class`] of the file was invalid.
    InvalidClass(u8),
    /// The [`Encoding`] of the file was invalid.
    InvalidEncoding(u8),
    /// The version is not [`ELF_VERSION`].
    InvalidVersion(u8),
    /// The byte slice was too small.
    TooSmall,
}

impl Display for ParseIdentError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ParseIdentError::IncorrectMagic(bytes) => write!(
                f,
                "magic bytes were {bytes:02X?}, expected {:02X?}",
                Ident::MAGIC,
            ),
            ParseIdentError::InvalidClass(byte) => {
                write!(f, "invalid class specifier {byte}, expected 1 or 2")
            }
            ParseIdentError::InvalidEncoding(byte) => {
                write!(f, "invalid encoding specifier {byte}, expected 1 or 2")
            }
            ParseIdentError::InvalidVersion(version) => {
                write!(f, "invalid elf version: {version}, expected {ELF_VERSION}")
            }
            ParseIdentError::TooSmall => {
                write!(
                    f,
                    "slice was too small to contain an ELF ident: must be at least 16 bytes"
                )
            }
        }
    }
}

impl Error for ParseIdentError {}

/// The capacity of the file. Determines size and layout of ELF structures.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Class {
    /// Supports machines with 32-bit architectures.
    Class32,
    /// Supports machines with 64-bit architectures.
    Class64,
}

impl Display for Class {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Class::Class32 => f.write_str("32-bit"),
            Class::Class64 => f.write_str("64-bit"),
        }
    }
}

/// Specifies the encoding of both the data strctures used by the object file container and data
/// contained in the object file sections.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Encoding {
    /// Indicates the data is encoded in little-endian format.
    LittleEndian,
    /// Indicates the data is encoded in big-endian format.
    BigEndian,
}

impl Display for Encoding {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Encoding::LittleEndian => f.write_str("little-endian"),
            Encoding::BigEndian => f.write_str("big-endian"),
        }
    }
}

/// Identifies the OS- or ABI-specific ELF extensions used by this file.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct OsAbi(pub u8);

impl OsAbi {
    /// No extensions or unspecified.
    pub const NONE: OsAbi = OsAbi(0);
    /// Hewlett-Packard HP-UX
    pub const HPUX: OsAbi = OsAbi(1);
    /// NetBSD
    #[expect(clippy::doc_markdown)]
    pub const NETBSD: OsAbi = OsAbi(2);
    /// GNU
    pub const GNU: OsAbi = OsAbi(3);
    /// Linux - historical
    pub const LINUX: OsAbi = OsAbi(3);
    /// Sun Solaris
    pub const SOLARIS: OsAbi = OsAbi(6);
    /// AIX
    pub const AIX: OsAbi = OsAbi(7);
    /// IRIX
    pub const IRIX: OsAbi = OsAbi(8);
    /// FreeBSD
    pub const FREEBSD: OsAbi = OsAbi(9);
    /// Compaq TRU64 UNIX
    pub const TRU64: OsAbi = OsAbi(10);
    /// Novell Modesto
    pub const MODESTO: OsAbi = OsAbi(11);
    /// Open BSD
    pub const OPENBSD: OsAbi = OsAbi(12);
    /// Open VMS
    pub const OPENVMS: OsAbi = OsAbi(13);
    /// Hewlett-Packard Non-Stop Kernel
    pub const NSK: OsAbi = OsAbi(14);
    /// Amiga Research OS
    pub const AROS: OsAbi = OsAbi(15);
    /// The FenixOS highly scalable multi-core OS
    #[expect(clippy::doc_markdown)]
    pub const FENIXOS: OsAbi = OsAbi(16);
    /// Nuxi CloudABI
    #[expect(clippy::doc_markdown)]
    pub const CLOUDABI: OsAbi = OsAbi(17);
    /// Stratus Technologies OpenVOS
    #[expect(clippy::doc_markdown)]
    pub const OPENVOS: OsAbi = OsAbi(18);
    /// Inclusive start of the architecture specific [`OsAbi`] range.
    pub const ARCHITECTURE_SPECIFIC_START: OsAbi = OsAbi(64);
    /// Inclusive end of the architecture specific [`OsAbi`] range.
    pub const ARCHITECTURE_SPECIFIC_END: OsAbi = OsAbi(255);

    /// Tests whether the meaning of this [`OsAbi`] is architecture specific.
    ///
    /// # Examples
    ///
    /// ```
    /// let arch_specific = OsAbi(65);
    /// let arch_independent = OsAbi::FREEBSD;
    ///
    /// assert!(arch_specific.is_architecture_specific())
    /// assert!(!arch_independent.is_architecture_specific());
    /// ```
    #[must_use]
    pub fn is_architecture_specific(&self) -> bool {
        &Self::ARCHITECTURE_SPECIFIC_START <= self && self <= &Self::ARCHITECTURE_SPECIFIC_END
    }
}

impl Display for OsAbi {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::NONE => f.write_str("none"),
            Self::HPUX => f.write_str("hpux"),
            Self::NETBSD => f.write_str("netbsd"),
            Self::GNU => f.write_str("gnu"),
            Self::SOLARIS => f.write_str("solaris"),
            Self::AIX => f.write_str("aix"),
            Self::IRIX => f.write_str("irix"),
            Self::FREEBSD => f.write_str("freebsd"),
            Self::TRU64 => f.write_str("true64"),
            Self::MODESTO => f.write_str("modesto"),
            Self::OPENBSD => f.write_str("openbsd"),
            Self::OPENVMS => f.write_str("openvms"),
            Self::NSK => f.write_str("nsk"),
            Self::AROS => f.write_str("aros"),
            Self::FENIXOS => f.write_str("fenixos"),
            Self::CLOUDABI => f.write_str("cloudabi"),
            Self::OPENVOS => f.write_str("openvos"),
            _ if self.is_architecture_specific() => {
                write!(f, "architecture({})", self.0)
            }
            _ => write!(f, "unknown({})", self.0),
        }
    }
}
