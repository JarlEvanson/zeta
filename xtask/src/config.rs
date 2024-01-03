use std::{
    fmt::{Debug, Display},
    io::{Read, Seek, Write},
    mem,
    ptr::NonNull,
};

use digest::sha512::Digest;

const PE_SIGNATURE: [u8; 4] = [b'P', b'E', b'\0', b'\0'];

const PE_PLUS_MAGIC: u16 = 0x020B;

const COFF_OPTIONAL_HEADER_OFFSET: usize = 24;

pub fn update_config_checksum<F>(mut file: F, digest: Digest) -> Result<(), UpdateChecksumError>
where
    F: Read + Write + Seek,
{
    let offset = {
        let mut offset = [0; 4];

        file.seek(std::io::SeekFrom::Start(0x3C))?;
        file.read_exact(&mut offset)?;

        u32::from_le_bytes(offset) as usize
    };

    let mut buffer = [0; 26];

    file.seek(std::io::SeekFrom::Start(offset as u64))?;
    file.read_exact(&mut buffer)?;

    if &buffer[0..4] != &PE_SIGNATURE {
        return Err(UpdateChecksumError::InvalidSignature);
    }

    let section_count = u16_from_slice(&buffer, 6) as usize;
    let optional_header_size = u16_from_slice(&buffer, 20) as usize;

    if u16_from_slice(&buffer, COFF_OPTIONAL_HEADER_OFFSET) != PE_PLUS_MAGIC {
        return Err(UpdateChecksumError::InvalidPEType);
    }

    let mut buffer = vec![0; section_count * mem::size_of::<SectionHeaderEntry>()];

    file.seek(std::io::SeekFrom::Start(
        (offset + COFF_OPTIONAL_HEADER_OFFSET + optional_header_size) as u64,
    ))?;
    file.read_exact(&mut buffer)?;

    let config_entry = 'find: {
        for index in 0..section_count as usize {
            let section_header_start = index * mem::size_of::<SectionHeaderEntry>();

            // Assert length
            let _ = &buffer
                [section_header_start..section_header_start + mem::size_of::<SectionHeaderEntry>()];

            let entry = unsafe {
                NonNull::from(&buffer[section_header_start])
                    .cast::<SectionHeaderEntry>()
                    .as_ptr()
                    .read_unaligned()
            };

            if entry.name() == ".config\0" {
                break 'find entry;
            }
        }

        return Err(UpdateChecksumError::MissingConfigSection);
    };

    assert_eq!(config_entry.virtual_size, 64);

    let bytes = unsafe { core::mem::transmute::<_, [u8; 64]>(digest.as_u64s()) };

    file.seek(std::io::SeekFrom::Start(
        config_entry.raw_data_pointer as u64,
    ))
    .unwrap();
    file.write_all(&bytes).unwrap();

    Ok(())
}

#[derive(Debug)]
pub enum UpdateChecksumError {
    Io(std::io::Error),
    InvalidSignature,
    InvalidPEType,
    MissingConfigSection,
}

impl From<std::io::Error> for UpdateChecksumError {
    fn from(value: std::io::Error) -> Self {
        UpdateChecksumError::Io(value)
    }
}

impl Display for UpdateChecksumError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateChecksumError::Io(err) => write!(f, "an io error occurred: {err}"),
            UpdateChecksumError::InvalidSignature => write!(f, "the PE signature was incorrect"),
            UpdateChecksumError::InvalidPEType => write!(f, "the PE type was not PE+"),
            UpdateChecksumError::MissingConfigSection => {
                write!(f, "the file was missing a .config section to update")
            }
        }
    }
}

fn u16_from_slice(buffer: &[u8], at: usize) -> u16 {
    ((buffer[at + 1] as u16) << 8) | (buffer[at] as u16)
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct SectionHeaderEntry {
    name: [u8; 8],
    virtual_size: u32,
    virtual_address: u32,
    raw_data_size: u32,
    raw_data_pointer: u32,
    relocation_pointer: u32,
    line_numbers: u32,
    relocation_count: u16,
    line_number_count: u16,
    characteristics: u32,
}

impl SectionHeaderEntry {
    pub fn name(&self) -> &str {
        core::str::from_utf8(&self.name).unwrap()
    }
}

impl Debug for SectionHeaderEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dstruct = f.debug_struct("SectionHeaderEntry");

        dstruct.field("name", &self.name());
        dstruct.field("virtual_size", &self.virtual_size);
        dstruct.field("virtual_address", &self.virtual_address);
        dstruct.field("raw_data_size", &self.raw_data_size);
        dstruct.field("raw_data_pointer", &self.raw_data_pointer);
        dstruct.field("relocation_pointer", &self.relocation_pointer);
        dstruct.field("line_numbers", &self.line_numbers);
        dstruct.field("relocation_count", &self.relocation_count);
        dstruct.field("line_number_count", &self.line_number_count);
        dstruct.field("characteristics", &self.characteristics);

        dstruct.finish()
    }
}
