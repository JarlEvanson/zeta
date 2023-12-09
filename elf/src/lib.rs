//! The `elf` crate provides pure rust interface for reading
//! ELF object files.

#![no_std]
#![feature(lint_reasons, error_in_core)]

use header::ParseHeaderError;
use ident::{Ident, ParseIdentError};

use crate::header::Header;

pub mod header;
pub mod ident;

/// The only ELF version this library supports.
pub const ELF_VERSION: u8 = 1;

/// An ELF file.
pub struct Elf<'file> {
    /// The underlying bytes of the ELF file.
    bytes: &'file [u8],
    /// The machine independent parts of the ELF file.
    ident: Ident,
}

impl<'file> Elf<'file> {
    /// Parses the ELF file as minimally as possible while still being able to
    /// validate that the major components of the file are valid.
    ///
    /// # Errors
    /// Returns errors if the ELF [`Ident`] or ELF [`Header`] failed to parse.
    pub fn parse(bytes: &'file [u8]) -> Result<Elf<'file>, ParseElfError> {
        let ident = Ident::parse(bytes)?;

        let elf = Elf { bytes, ident };

        Header::parse(&elf)?;

        Ok(elf)
    }

    /// Returns the parsed ELF [`Ident`].
    #[must_use]
    pub fn ident(&self) -> Ident {
        self.ident
    }

    /// Returns the parsed ELF [`Header`].
    #[must_use]
    pub fn header(&self) -> Header<'file> {
        // SAFETY:
        // During construction, header was parsed without error.
        unsafe { Header::parse(self).unwrap_unchecked() }
    }
}

/// Various errors which can occur while doing a minimal parse of an ELF file.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParseElfError {
    /// An error parsing the ELF [`Ident`].
    IdentError(ParseIdentError),
    /// An error parsing the ELF [`Header`].
    HeaderError(ParseHeaderError),
}

impl From<ParseIdentError> for ParseElfError {
    fn from(value: ParseIdentError) -> Self {
        ParseElfError::IdentError(value)
    }
}

impl From<ParseHeaderError> for ParseElfError {
    fn from(value: ParseHeaderError) -> Self {
        ParseElfError::HeaderError(value)
    }
}
