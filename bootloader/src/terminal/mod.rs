//! Contains the implementation of a graphical terminal and associated
//! code.

use framebuffer::Framebuffer;

use self::psf::{Font, Glyph};

pub mod framebuffer;
pub mod info;
pub mod psf;

/// Allows logging text to a pixel-based framebuffer.
#[derive(Debug)]
pub struct Terminal<'buffer, 'font> {
    /// The underlying framebuffer.
    framebuffer: Framebuffer<'buffer>,
    /// The font used to output characters.
    font: Font<'font>,
    /// The base coordinates in the top left corner.
    base: PixelCoordinates,
    /// The number of rows in the terminal.
    line_count: usize,
    /// The number of columns in the terminal.
    column_count: usize,
    /// The width of a glyph and additional padding.
    pixels_per_glyph: usize,
    /// The height of a glyph and additional padding.
    rows_per_glyph: usize,
    /// Position to render next character.
    next_write: CharacterCoordinates,
    /// The color of the characters.
    pub fg: Color,
    /// The color of the background.
    pub bg: Color,
}

impl<'buffer, 'font> Terminal<'buffer, 'font> {
    /// Minimum glyph column count.
    pub const MINIMUM_COLUMNS: usize = 10;
    /// Minimum glyph row count.
    pub const MINIMUM_ROWS: usize = 10;

    /// Creates a new terminal that uses `framebuffer`.
    pub fn new(
        mut framebuffer: Framebuffer<'buffer>,
        font: Font<'font>,
        formatting: Formatting,
        fg: Color,
        bg: Color,
    ) -> Result<Self, CreateTerminalError> {
        let total_char_width = font
            .glyph_width()
            .checked_add(formatting.spacing.char_padding)
            .ok_or(CreateTerminalError::InvalidFormatting)?;

        let (column_count, x_pixel_remainder) = compute_padding(
            framebuffer.info().width(),
            Self::MINIMUM_COLUMNS,
            total_char_width,
            formatting.padding.x_pixels,
        )?;

        let total_row_height = font
            .glyph_height()
            .checked_add(formatting.spacing.row_padding)
            .ok_or(CreateTerminalError::InvalidFormatting)?;

        let (row_count, y_pixel_remainder) = compute_padding(
            framebuffer.info().height(),
            Self::MINIMUM_ROWS,
            total_row_height,
            formatting.padding.y_pixels,
        )?;

        let base = {
            let x = formatting.padding.x_pixels + x_pixel_remainder / 2;
            let y = formatting.padding.y_pixels + y_pixel_remainder / 2;
            PixelCoordinates { x, y }
        };

        framebuffer.clear(Color {
            r: 0x00,
            g: 0x00,
            b: 0x00,
        });

        let terminal = Self {
            framebuffer,
            font,
            base,
            line_count: row_count,
            column_count,
            pixels_per_glyph: total_char_width,
            rows_per_glyph: total_row_height,
            next_write: CharacterCoordinates { x: 0, y: 0 },
            fg,
            bg,
        };

        log::info!("{:#?}", terminal);

        Ok(terminal)
    }
    /// Returns access to the underlying framebuffer.
    pub fn framebuffer(&self) -> &Framebuffer<'buffer> {
        &self.framebuffer
    }

    /// Returns the number of glyph lines are in the framebuffer.
    pub fn line_count(&self) -> usize {
        self.line_count
    }

    /// Returns the number of glpyh columns are in the framebuffer.
    pub fn column_count(&self) -> usize {
        self.column_count
    }

    /// Clears the screen.
    pub fn clear(&mut self) {
        self.framebuffer.clear(self.bg);
    }

    /// Moves the cursor to a new line, scrolling if necessary.
    pub fn newline(&mut self) {
        let new_ypos = self.next_write.y + 1;

        if new_ypos >= self.line_count {
            self.scroll((new_ypos + 1) - self.line_count);
            return;
        }

        self.next_write.y = new_ypos;
    }

    /// Moves the cursor to the next cell, scrolling if necessary.
    fn next_char(&mut self) {
        let new_xpos = self.next_write.x + 1;

        if new_xpos >= self.column_count {
            self.newline();
            self.next_write.x = 0;
            return;
        }

        self.next_write.x = new_xpos;
    }

    /// Copies lines up `lines` and clears the bottom `lines`.
    pub fn scroll(&mut self, lines: usize) {
        if lines >= self.line_count {
            self.clear();
            return;
        } else if lines == 0 {
            return;
        }

        let dest = PixelCoordinates {
            x: 0,
            y: self.base.y,
        };

        let src = Rectangle {
            top_left: PixelCoordinates {
                x: 0,
                y: self.base.y + self.rows_per_glyph * lines,
            },
            height: (self.line_count - lines) * self.rows_per_glyph,
            width: self.framebuffer.info().width(),
        };

        self.framebuffer.copy_within(src, dest).unwrap();

        let obsolete_rows = Rectangle {
            top_left: PixelCoordinates {
                x: 0,
                y: self.base.y + (self.line_count - lines) * self.rows_per_glyph,
            },
            height: self.framebuffer.info().height()
                - (self.base.y + (self.line_count - lines) * self.rows_per_glyph),
            width: self.framebuffer.info().width(),
        };

        self.framebuffer.fill(obsolete_rows, self.bg).unwrap();
    }

    /// Writes a character at the next cell.
    pub fn write_codepoint(&mut self, c: char) {
        match c {
            '\n' => {
                self.newline();
                self.next_write.x = 0;
            }
            '\r' => self.next_write.x = 0,
            c => {
                if let Some(glyph) = self.font.lookup(c) {
                    self.write_glyph(glyph, self.next_write);
                } else if let Some(glyph) = self.font.lookup('?') {
                    core::mem::swap(&mut self.fg, &mut self.bg);

                    self.write_glyph(glyph, self.next_write);

                    core::mem::swap(&mut self.fg, &mut self.bg);
                } else {
                    let coords = PixelCoordinates {
                        x: self.base.x + self.next_write.x * self.pixels_per_glyph,
                        y: self.base.y + self.next_write.y * self.rows_per_glyph,
                    };

                    let glyph_rect = Rectangle {
                        top_left: coords,
                        width: self.pixels_per_glyph,
                        height: self.rows_per_glyph,
                    };

                    let _ = self.framebuffer.fill(glyph_rect, self.fg);
                }

                self.next_char();
            }
        }
    }

    /// Writes a glyph with the top left corner being `(x_pos, y_pos)`.
    ///
    /// Assumes the rectangle formed by `(x_pos, y_pos)` and
    /// `(x_pos + PSF_FONT.width() - 1, y_pos + PSF_FONT_HEIGHT - 1)`
    /// is within the visible framebuffer.
    fn write_glyph(&mut self, glyph: Glyph, coords: CharacterCoordinates) {
        let coords = PixelCoordinates {
            x: self.base.x + coords.x * self.pixels_per_glyph,
            y: self.base.y + coords.y * self.rows_per_glyph,
        };

        for (y_offset, row) in glyph.rows().enumerate() {
            for (x_offset, byte) in row.enumerate() {
                let color = if byte { self.fg } else { self.bg };
                let _ = self.framebuffer.write_pixel(
                    PixelCoordinates {
                        x: coords.x + x_offset,
                        y: coords.y + y_offset,
                    },
                    color,
                );
            }
        }
    }
}

impl core::fmt::Write for Terminal<'_, '_> {
    fn write_char(&mut self, c: char) -> core::fmt::Result {
        self.write_codepoint(c);

        Ok(())
    }

    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.chars() {
            self.write_codepoint(c);
        }

        Ok(())
    }
}

/// Computes the padding and the total number of items a dimension can
/// hold.
fn compute_padding(
    dimension_size: usize,
    min_count: usize,
    spacing: usize,
    padding: usize,
) -> Result<(usize, usize), CreateTerminalError> {
    let Some(total_padding) = padding.checked_mul(2) else {
        return Err(CreateTerminalError::InvalidFormatting);
    };

    let Some(unpadded_pixels) = dimension_size.checked_sub(total_padding) else {
        return Err(CreateTerminalError::InvalidFormatting);
    };

    let count = match unpadded_pixels.checked_div(spacing) {
        Some(count) if count >= min_count => count,
        None => return Err(CreateTerminalError::InvalidFormatting),
        _ => return Err(CreateTerminalError::FramebufferTooSmall),
    };

    let Some(remaining_pixels) = unpadded_pixels.checked_rem(spacing) else {
        return Err(CreateTerminalError::UnsupportedFont);
    };

    Ok((count, remaining_pixels))
}

/// Various errors that can occur while creating a terminal.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum CreateTerminalError {
    /// The formatting was invalid.
    InvalidFormatting,
    /// The framebuffer was too small for the required font and formatting.
    FramebufferTooSmall,
    /// The font was invalid.
    UnsupportedFont,
}

/// The location of a pixel in a framebuffer.
///
/// Starts from the top-left of the framebuffer.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct PixelCoordinates {
    /// The x component of pixel location.
    pub x: usize,
    /// The y component of pixel location.
    pub y: usize,
}

/// The location of character in a terminal.
///
/// Starts from the top-left of the terminal.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub(crate) struct CharacterCoordinates {
    /// The x component of character location.
    pub x: usize,
    /// The y component of character location.
    pub y: usize,
}

/// A RGB color.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct Color {
    /// The intensity of the red color.
    pub r: u8,
    /// The intensity of the green color.
    pub g: u8,
    /// The intensity of the blue color.
    pub b: u8,
}

/// A rectangle in the pixel space.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct Rectangle {
    /// Top-left corner of the rectangle.
    pub top_left: PixelCoordinates,
    /// Width of the rectangle in pixels.
    pub width: usize,
    /// Height of the rectange in pixels.
    pub height: usize,
}

/// Options for formatting of the [`Terminal`].
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct Formatting {
    /// The padding settings.
    pub padding: BorderPadding,
    /// The spacing settings.
    pub spacing: Spacing,
}

/// Padding in pixels for a [`Terminal`].
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct BorderPadding {
    /// Additional padding from the x-border to prevent fonts that are too close
    /// to the border.
    pub x_pixels: usize,
    /// Additional padding from the y-border to prevent fonts that are too close
    /// to the border.
    pub y_pixels: usize,
}

/// Spacing between rows and columns in pixels for a [`Terminal`].
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct Spacing {
    /// Additional vertical space between lines in pixels.
    pub row_padding: usize,
    /// Additional horizontal space between characters in pixels.
    pub char_padding: usize,
}
