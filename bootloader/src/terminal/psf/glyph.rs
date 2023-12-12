//! Contains the implementation of PC Screen Font glyph iterators.

use core::fmt::{Display, Write};

/// A PC Screen Font glpyh.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Glyph<'a> {
    /// The buffer storing the pixel values of a glyph.
    data: &'a [u8],
    /// The width of each glyph in pixels.
    width: usize,
}

impl<'a> Glyph<'a> {
    /// Creates a new [`Glyph`].
    pub(crate) fn new(data: &'a [u8], width: usize) -> Glyph<'a> {
        Glyph { data, width }
    }

    /// Returns an iterator over the rows of pixels in `self`.
    pub fn rows(&'a self) -> Iter<'a> {
        Iter::new(self.data, self.width)
    }
}

impl Display for Glyph<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for row in self.rows() {
            for b in row {
                if b {
                    f.write_char('#')?;
                } else {
                    f.write_char('.')?;
                }
            }
            f.write_char('\n')?;
        }

        Ok(())
    }
}

/// An iterator of the rows of a glyph.
#[derive(Clone, PartialEq, Eq)]
pub struct Iter<'a> {
    /// The buffer storing the pixel values of a glyph.
    data: &'a [u8],
    /// The width of each glyph in pixels.
    width: usize,
}

impl<'a> Iter<'a> {
    /// Creates a new iterator over the rows of a glyph according to the PC
    /// Screen Font specification.
    pub(crate) fn new(data: &'a [u8], width: usize) -> Iter<'a> {
        Self { data, width }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = Row<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let advance = self.width.div_ceil(8);
        if self.data.len() < advance {
            return None;
        }

        let next = &self.data[..advance];
        self.data = &self.data[advance..];
        Some(Row::new(next, self.width))
    }
}

/// An iterator over the pixels in a glyph row.
#[derive(Clone, PartialEq, Eq)]
pub struct Row<'a> {
    /// The buffer storing the row's pixel values.
    data: &'a [u8],
    /// The number of pixels in the row.
    width: usize,
    /// The index of the next pixel value to output.
    next: usize,
}

impl<'a> Row<'a> {
    /// Creates a new iterator over the pixels in a glyph row.
    pub(crate) fn new(data: &'a [u8], width: usize) -> Row<'a> {
        assert!(((width - 1) >> 3) < data.len());
        Self {
            data,
            width,
            next: 0,
        }
    }
}

impl<'a> Iterator for Row<'a> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next >= self.width {
            return None;
        }

        // SAFETY:
        // On creation, `self.width` >> 3 < `self.data.len()`.
        // The guard statement makes sure that `self.next` < `self.width`
        let byte = unsafe { self.data.get_unchecked(self.next >> 3) };

        #[allow(clippy::cast_possible_truncation)]
        let mask = 1 << (7 - (self.next as u8 & 0x7));

        let result = byte & mask == mask;

        self.next += 1;
        Some(result)
    }
}
