//! Contains the implementation of [`Info`] and its validation.

/// Information required to properly utilize a framebuffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Info {
    /// The total size of the framebuffer in bytes.
    ///
    /// By the invariants of this struct, is at least as large as
    /// `((height - 1) * stride + width) * bytes_per_pixel`.
    byte_len: usize,
    /// The width in pixels.
    width: usize,
    /// The height in pixels.
    height: usize,
    /// The format of each pixel.
    format: PixelFormat,
    /// The number of bytes per pixel.
    bytes_per_pixel: usize,
    /// The number of pixels betwee the start of a line and the start of the next.
    ///
    /// By the invariants of this struct, must be greater than or equal to `width`.
    stride: usize,
}

impl Info {
    /// Creates an [`Info`], validating that the basic invariants are upheld.
    ///
    /// # Errors
    /// - [`IllegalStride`][is]
    ///     - Returned when `stride` is less than `width`.
    /// - [`IllegalBytesPerPixel`][ibpp]
    ///     - Returned when `bytes_per_pixel` is less than `format`'s minimum bytes per pixel.
    /// - [`IllegalBufferSize`][ibs]
    ///     - Returned when the buffer is smaller than the dimensions would required.
    /// - [`Overflow`][o]
    ///     - Returned when the buffer is too large for the address space.
    pub fn new(
        byte_len: usize,
        width: usize,
        height: usize,
        format: PixelFormat,
        bytes_per_pixel: usize,
        stride: usize,
    ) -> Result<Info, ValidateInfoError> {
        if stride < width {
            return Err(ValidateInfoError::IllegalStride {
                actual: stride,
                minimum: width,
            });
        }

        if bytes_per_pixel < 4 {
            return Err(ValidateInfoError::IllegalBytesPerPixel {
                actual: bytes_per_pixel,
                minimum: 4,
            });
        }

        // Check that `byte_len` meets the invariants of this struct.
        if let Some(strides_needed) = height.checked_sub(1) {
            match strides_needed
                .checked_mul(stride)
                .and_then(|pixels| pixels.checked_add(width))
                .and_then(|pixels| pixels.checked_mul(bytes_per_pixel))
            {
                Some(minimum_bytes)
                    if minimum_bytes <= byte_len && isize::try_from(byte_len).is_ok() => {}
                Some(minimum_bytes) => {
                    return Err(ValidateInfoError::IllegalBufferSize {
                        actual: byte_len,
                        minimum: minimum_bytes,
                    })
                }
                None => return Err(ValidateInfoError::Overflow),
            }
        };

        let info = Self {
            byte_len,
            width,
            height,
            format,
            bytes_per_pixel,
            stride,
        };

        Ok(info)
    }

    /// Returns the number of bytes in the frambuffer.
    ///
    /// Will always be less than or equal to [`isize::MAX`]
    #[inline]
    pub const fn size(&self) -> usize {
        self.byte_len
    }

    /// Returns the width of the buffer in pixels.
    ///
    /// Will always be less than [`stride()`][s].
    ///
    /// [s]: Self::width
    #[inline]
    pub const fn width(&self) -> usize {
        self.width
    }

    /// Returns the height of the buffer in pixels.
    #[inline]
    pub const fn height(&self) -> usize {
        self.height
    }

    /// Returns the format of the pixels in the buffer.
    #[inline]
    pub const fn format(&self) -> PixelFormat {
        self.format
    }

    /// Returns the number of bytes each pixel occupies.
    #[inline]
    pub const fn bytes_per_pixel(&self) -> usize {
        self.bytes_per_pixel
    }

    /// Returns the number of pixels between the start of each line.
    #[inline]
    pub const fn stride(&self) -> usize {
        self.stride
    }
}

/// Various errors returned when validating an [`Info`].
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValidateInfoError {
    /// The stride was smaller than the width.
    IllegalStride {
        /// The passed stride.
        actual: usize,
        /// The minimum stride.
        minimum: usize,
    },
    /// The bytes per pixel was smaller than the minimum for the format.
    IllegalBytesPerPixel {
        /// The passed bytes per pixel.
        actual: usize,
        /// The minimum bytes per pixel.
        minimum: usize,
    },
    /// The stated buffer length was smaller than the dimensions.
    IllegalBufferSize {
        /// The passed buffer length.
        actual: usize,
        /// The minimum buffer length according to the dimensions.
        minimum: usize,
    },
    /// An overflow occurred while validating the buffer size.
    Overflow,
}

/// Recognized pixel formats.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum PixelFormat {
    /// A u32 value with bits 0..8 controlling the intensity of red, bits 8..16
    /// controlling the intensity of green, and bits 16..24 controlling the intensity
    /// of blue.  
    Rgb,
    /// A u32 value with bits 0..8 controlling the intensity of blue, bits 8..16
    /// controlling the intensity of green, and bits 16..24 controlling the intensity
    /// of red.  
    Bgr,
}

#[cfg(test)]
mod test {
    use super::{Info, PixelFormat};

    #[test]
    #[should_panic(expected = "Stride")]
    fn smaller_stride() {
        let width = 50;
        let height = 50;
        let format = PixelFormat::Rgb;
        let bytes_per_pixel = 4;
        let stride = 49;
        let byte_len = ((height - 1) * stride + width) * bytes_per_pixel;

        Info::new(byte_len, width, height, format, bytes_per_pixel, stride).unwrap();
    }

    #[test]
    #[should_panic(expected = "Bytes")]
    fn fewer_bytes() {
        let width = 50;
        let height = 50;
        let format = PixelFormat::Rgb;
        let bytes_per_pixel = 3;
        let stride = 50;
        let byte_len = ((height - 1) * stride + width) * bytes_per_pixel;

        Info::new(byte_len, width, height, format, bytes_per_pixel, stride).unwrap();
    }

    #[test]
    #[should_panic(expected = "BufferSize")]
    fn smaller_buffer() {
        let width = 50;
        let height = 50;
        let format = PixelFormat::Rgb;
        let bytes_per_pixel = 4;
        let stride = 50;
        let byte_len = ((height - 1) * stride + width) * bytes_per_pixel - 1;

        Info::new(byte_len, width, height, format, bytes_per_pixel, stride).unwrap();
    }

    #[test]
    #[should_panic(expected = "Overflow")]
    fn overflow_testing() {
        let width = 50;
        let height = usize::MAX / 2;
        let format = PixelFormat::Rgb;
        let bytes_per_pixel = 4;
        let stride = 50;
        // For this test, doesn't matter.
        let byte_len = 0;

        Info::new(byte_len, width, height, format, bytes_per_pixel, stride).unwrap();
    }

    #[test]
    fn success() {
        let width = 50;
        let height = 50;
        let format = PixelFormat::Rgb;
        let bytes_per_pixel = 4;
        let stride = 50;
        // For this test, doesn't matter.
        let byte_len = usize::MAX / 2;

        Info::new(byte_len, width, height, format, bytes_per_pixel, stride).unwrap();
    }
}
