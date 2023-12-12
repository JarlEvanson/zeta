//! Contains the implementation of a compile-time constructed unicode
//! hash table for a PC Screen Font.

mod glyph;

use core::fmt::Debug;

pub use glyph::*;

/// Bytes containing a PC Screen Font version 2.
const PSF_FONT_BYTES: &[u8; 4677] = include_bytes!("../../../../assets/Tamsyn8x16r.psf");

/// Parsed hashmap for the font contained in [`PSF_FONT_BYTES`].
pub const FONT: Font = {
    if PSF_FONT_BYTES[0] != 0x72
        || PSF_FONT_BYTES[1] != 0xb5
        || PSF_FONT_BYTES[2] != 0x4a
        || PSF_FONT_BYTES[3] != 0x86
    {
        panic!("Invalid magic");
    }

    {
        let ptr = unsafe { PSF_FONT_BYTES.as_ptr().add(4) };

        let version = unsafe { u32::from_le(ptr.cast::<u32>().read_unaligned()) };

        #[allow(clippy::manual_assert)]
        if version != 0 {
            panic!("Invalid version");
        }
    };

    let header_size = {
        let ptr = unsafe { PSF_FONT_BYTES.as_ptr().add(8) };

        unsafe { u32::from_le(ptr.cast::<u32>().read_unaligned()) }
    } as usize;

    // Assert that this font has a unicode table.
    {
        let ptr = unsafe { PSF_FONT_BYTES.as_ptr().add(12) };

        let flags = unsafe { u32::from_le(ptr.cast::<u32>().read_unaligned()) };

        #[allow(clippy::manual_assert)]
        if flags & 0x1 == 0x0 {
            panic!("non-unicode table not supported");
        }
    };

    let glyph_count = {
        let ptr = unsafe { PSF_FONT_BYTES.as_ptr().add(16) };

        unsafe { u32::from_le(ptr.cast::<u32>().read_unaligned()) }
    } as usize;

    let bytes_per_glyph = {
        let ptr = unsafe { PSF_FONT_BYTES.as_ptr().add(20) };

        unsafe { u32::from_le(ptr.cast::<u32>().read_unaligned()) }
    } as usize;

    let glyph_height = {
        let ptr = unsafe { PSF_FONT_BYTES.as_ptr().add(24) };

        unsafe { u32::from_le(ptr.cast::<u32>().read_unaligned()) }
    } as usize;

    let glyph_width = {
        let ptr = unsafe { PSF_FONT_BYTES.as_ptr().add(28) };

        unsafe { u32::from_le(ptr.cast::<u32>().read_unaligned()) }
    } as usize;

    let glyphs: &'static [u8] = {
        let glyph_storage =
            unsafe { core::intrinsics::const_allocate(glyph_count * bytes_per_glyph, 1) };

        let glyph_base = unsafe { PSF_FONT_BYTES.as_ptr().add(header_size) };

        unsafe {
            core::ptr::copy_nonoverlapping(
                glyph_base,
                glyph_storage,
                glyph_count * bytes_per_glyph,
            );
        }

        unsafe { core::slice::from_raw_parts(glyph_storage, glyph_count * bytes_per_glyph) }
    };

    let unicode_table_offset = header_size + glyph_count * bytes_per_glyph;

    let entry_count = {
        let mut entries: usize = 0;

        let mut current_offset = unicode_table_offset;

        while current_offset < PSF_FONT_BYTES.len() {
            let byte = PSF_FONT_BYTES[current_offset];

            if byte == 0xFF {
                current_offset += 1;
                continue;
            } else if byte == 0xFE {
                panic!("does not support codepoint sequences");
            }

            entries += 1;

            current_offset =
                current_offset + UTF8_CHAR_WIDTH[PSF_FONT_BYTES[current_offset] as usize] as usize;
        }

        entries
    };

    #[allow(clippy::cast_possible_truncation)]
    let capacity = ((entry_count * 8) / 7).next_power_of_two() as u32;
    let capacity_bitmask = capacity - 1;

    #[allow(clippy::cast_ptr_alignment)]
    let buffer = unsafe {
        core::intrinsics::const_allocate(
            capacity as usize * core::mem::size_of::<Option<Entry>>(),
            core::mem::align_of::<Option<Entry>>(),
        )
        .cast::<Option<Entry>>()
    };

    {
        let mut index = 0;

        while index < capacity {
            let out_ptr = unsafe { buffer.add(index as usize) };

            unsafe {
                out_ptr.write(None);
            }

            index += 1;
        }
    }

    {
        let mut current_glyph = 0;

        let mut current_offset = unicode_table_offset;

        while current_offset < PSF_FONT_BYTES.len() {
            let byte = PSF_FONT_BYTES[current_offset];

            if byte == 0xFF {
                current_offset += 1;
                current_glyph += 1;
                continue;
            } else if byte == 0xFE {
                panic!("does not support codepoint sequences");
            }

            let c = match UTF8_CHAR_WIDTH[byte as usize] {
                1 => byte as u32,
                2 => {
                    let b_0 = PSF_FONT_BYTES[current_offset] as u32 & 0x1F;
                    let b_1 = PSF_FONT_BYTES[current_offset + 1] as u32 & 0x3F;

                    (b_0 << 6) | b_1
                }
                3 => {
                    let b_0 = PSF_FONT_BYTES[current_offset] as u32 & 0xF;
                    let b_1 = PSF_FONT_BYTES[current_offset + 1] as u32 & 0x3F;
                    let b_2 = PSF_FONT_BYTES[current_offset + 2] as u32 & 0x3F;

                    (b_0 << 12) | (b_1 << 6) | b_2
                }
                4 => {
                    let b_0 = PSF_FONT_BYTES[current_offset] as u32 & 0x7;
                    let b_1 = PSF_FONT_BYTES[current_offset + 1] as u32 & 0x3F;
                    let b_2 = PSF_FONT_BYTES[current_offset + 2] as u32 & 0x3F;
                    let b_3 = PSF_FONT_BYTES[current_offset + 3] as u32 & 0x3F;

                    (b_0 << 18) | (b_1 << 12) | (b_2 << 6) | b_3
                }
                _ => unreachable!(),
            };

            let Some(c) = char::from_u32(c) else {
                panic!("invalid unicode codepoint");
            };

            let entry = Entry {
                key: c,
                glyph_entry: current_glyph,
            };

            let mut stride = 0;
            let mut pos = hash_char(c) & capacity_bitmask;

            loop {
                let test_ptr = unsafe { buffer.add(pos as usize) };

                let val = unsafe { test_ptr.read() };

                if val.is_some() {
                    stride += 1;
                    pos += stride;
                    pos &= capacity_bitmask;
                    continue;
                }

                unsafe {
                    test_ptr.write(Some(entry));
                }
                break;
            }

            current_offset =
                current_offset + UTF8_CHAR_WIDTH[PSF_FONT_BYTES[current_offset] as usize] as usize;
        }
    }

    let buffer = unsafe { core::slice::from_raw_parts(buffer, capacity as usize) };

    Font {
        glyphs,
        #[allow(clippy::cast_possible_truncation)]
        bytes_per_glyph: bytes_per_glyph as u32,
        glyph_width,
        glyph_height,
        buffer,
    }
};

/// Table for how long a UTF-8 codepoint is based on its first byte.
const UTF8_CHAR_WIDTH: &[u8; 256] = &[
    // 1  2  3  4  5  6  7  8  9  A  B  C  D  E  F
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 0
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 1
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 2
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 3
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 4
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 5
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 6
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 7
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 8
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 9
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // A
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // B
    0, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, // C
    2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, // D
    3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, // E
    4, 4, 4, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // F
];

/// Hash function for the font table.
const fn hash_char(c: char) -> u32 {
    let mut x = (((c as u32) >> 16) ^ (c as u32)).wrapping_mul(0x045d_9f3b);
    x = (((c as u32) >> 16) ^ x).wrapping_mul(0x045d_9f3b);
    (x >> 16) ^ x
}

/// Data needed for triangular probing.
struct ProbeSeq {
    /// The current position to probe.
    pos: u32,
    /// The stride to the next position to probe.
    stride: u32,
}

impl ProbeSeq {
    /// Creates a new [`ProbeSeq`].
    fn new(pos: u32) -> ProbeSeq {
        ProbeSeq { pos, stride: 0 }
    }

    /// Updates info to next probing location.
    fn move_next(&mut self, table_size: u32) {
        self.stride += 1;
        self.pos += self.stride;
        self.pos &= table_size;
    }
}

/// A PC Screen Font.
pub struct Font<'font> {
    /// The data of the glyphs.
    glyphs: &'font [u8],
    /// Bytes per glyph.
    bytes_per_glyph: u32,
    /// Width of the glyph in pixels.
    glyph_width: usize,
    /// Height of the glyph in pixels.
    glyph_height: usize,
    /// The buffer used as the charcter-glyph hash table.
    buffer: &'font [Option<Entry>],
}

impl<'font> Font<'font> {
    /// Searches for `c` in the font table.
    pub fn lookup(&self, c: char) -> Option<Glyph<'font>> {
        #[allow(clippy::cast_possible_truncation)]
        let capacity_bitmask = (self.buffer.len() - 1) as u32;

        let mut probe_seq = ProbeSeq::new(hash_char(c) & capacity_bitmask);

        loop {
            let entry = self.buffer[probe_seq.pos as usize]?;

            if entry.key == c {
                let start_byte = self.bytes_per_glyph * entry.glyph_entry;
                let end_byte = start_byte + self.bytes_per_glyph;

                let glyph = Glyph::new(
                    &self.glyphs[start_byte as usize..end_byte as usize],
                    self.glyph_width,
                );

                return Some(glyph);
            }

            probe_seq.move_next(capacity_bitmask);
        }
    }

    /// Returns the number of glyphs in the [`Font`].
    pub fn glyph_count(&self) -> usize {
        self.glyphs.len() / (self.bytes_per_glyph as usize)
    }

    /// Returns the width of each [`Glyph`] in pixels.
    pub fn glyph_width(&self) -> usize {
        self.glyph_width
    }

    /// Returns the height of each [`Glyph`] in pixels.
    pub fn glyph_height(&self) -> usize {
        self.glyph_height
    }
}

impl Debug for Font<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut dstruct = f.debug_struct("Font");

        dstruct.field("glyph_count", &self.glyph_count());
        dstruct.field("bytes_per_glyph", &self.bytes_per_glyph);
        dstruct.field("glyph_width", &self.glyph_width);
        dstruct.field("glyph_height", &self.glyph_height);

        dstruct.finish_non_exhaustive()
    }
}

/// An entry into a [`Font`] character-glyph hash table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Entry {
    /// The key this entry.
    key: char,
    /// The index of the glyph this character corresponds to.
    glyph_entry: u32,
}
