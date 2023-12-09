//! The `elf` crate provides pure rust interface for reading
//! ELF object files.

#![no_std]
#![feature(lint_reasons, error_in_core)]

use ident::{Ident, ParseIdentError};

pub mod ident;

/// The only ELF version this library supports.
pub const ELF_VERSION: u8 = 1;

pub struct Elf<'file> {
    /// The underlying bytes of the ELF file.
    bytes: &'file [u8],
    /// The machine independent parts of the ELF file.
    ident: Ident,
}

impl<'file> Elf<'file> {
    /// Parses the elf file as minimally as possible while still being able to 
    /// validate that the major components of the file are valid.
    pub fn parse(bytes: &[u8]) -> Result<Elf<'file>, ParseElfError> {
        let ident = Ident::parse(bytes)?;

        todo!()
    }
}

pub enum ParseElfError {
    IdentError(ParseIdentError),
}

impl From<ParseIdentError> for ParseElfError {
    fn from(value: ParseIdentError) -> Self {
        ParseElfError::IdentError(value)
    }
}