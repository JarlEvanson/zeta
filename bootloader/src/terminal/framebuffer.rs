//! Contains the implementation of [`Framebuffer`], its validation, and buffer
//! manipulation methods.

use core::marker::PhantomData;

use super::{
    info::{Info, PixelFormat},
    Color, PixelCoordinates, Rectangle,
};

/// Representation of a writable framebuffer. Contains all the information
/// needed to safely write to a buffer.
pub struct Framebuffer<'buffer> {
    /// Pointer to the top-left corner of the framebuffer.
    ptr: *mut u8,
    /// Info corresponding to the framebuffer layout.
    info: Info,
    /// Here to simplify lifetimes.
    phantom: PhantomData<&'buffer mut [u8]>,
}

impl<'buffer> Framebuffer<'buffer> {
    /// Creates a new [`Framebuffer`], validating its length.
    ///
    /// # Errors
    /// Returns `None` if `framebuffer` was less than `info.size()`.
    pub fn new(framebuffer: &'buffer mut [u8], info: Info) -> Option<Framebuffer<'buffer>> {
        if framebuffer.len() < info.size() {
            return None;
        }

        let ptr = framebuffer.as_mut_ptr();

        let framebuffer = Framebuffer {
            ptr,
            info,
            phantom: PhantomData,
        };

        Some(framebuffer)
    }

    /// Returns information regarding the layout of the [`Framebuffer`].
    pub fn info(&self) -> Info {
        self.info
    }

    /// Sets the pixel at `(x, y)` to `color`.
    #[must_use]
    pub fn write_pixel(&mut self, coords: PixelCoordinates, color: Color) -> Option<()> {
        if coords.x >= self.info.width() || coords.y >= self.info.height() {
            return None;
        }

        // SAFETY:
        // - `x` is less than `self.info().width()`.
        // - `y` is less than `self.info().height()`.
        unsafe { self.write_pixel_unchecked(coords, color) }

        Some(())
    }

    /// Sets the pixel at `(x, y)` to `color`.
    ///
    /// # Safety
    /// - `x` is less than `self.info().width()`.
    /// - `y` is less than `self.info().height()`.
    #[allow(clippy::many_single_char_names)]
    pub unsafe fn write_pixel_unchecked(&mut self, coords: PixelCoordinates, color: Color) {
        let (r, g, b) = self.setup_rgb_pointers(coords);

        // SAFETY:
        // - Since `x` and `y` are in bounds by the invariants of this function, so to
        //  is `r`.
        // - `r` is valid for writes since we have a mutable buffer to it, and `u8` is
        //  always aligned.
        unsafe {
            r.write_volatile(color.r);
        }
        // SAFETY:
        // Since `x` and `y` are in bounds by the invariants of this function, so to
        // is `g`.
        // `g` is valid for writes since we have a mutable buffer to it, and `u8` is
        // always aligned.
        unsafe {
            g.write_volatile(color.g);
        }
        // SAFETY:
        // Since `x` and `y` are in bounds by the invariants of this function, so to
        // is `b`.
        // `b` is valid for writes since we have a mutable buffer to it, and `u8` is
        // always aligned.
        unsafe {
            b.write_volatile(color.b);
        }
    }

    /// Fills the pixels contained in `rectangle` with `color`.
    #[must_use]
    pub fn fill(&mut self, color: Color, rectangle: Rectangle) -> Option<()> {
        if let Some(max_x) = rectangle.top_left.x.checked_add(rectangle.width) {
            if max_x > self.info.width() {
                return None;
            }
        }

        if let Some(max_height) = rectangle.top_left.y.checked_add(rectangle.height) {
            if max_height > self.info.height() {
                return None;
            }
        }

        if rectangle.width == 0 || rectangle.height == 0 {
            return None;
        }
        // SAFETY:
        // - `rectange.top_left.x + rectangle.width > self.info().width()`.
        // - `rectange.top_left.y + rectangle.height > self.info().height()`.
        // - `rectange.width != 0`.
        // - `rectange.height != 0`.
        unsafe {
            self.fill_unchecked(color, rectangle);
        }

        Some(())
    }

    /// Fills the pixels contained in `rectangle` with `color`.
    ///
    /// # Safety
    /// - `rectange.top_left.x + rectangle.width > self.info().width()`.
    /// - `rectange.top_left.y + rectangle.height > self.info().height()`.
    /// - `rectange.width != 0`.
    /// - `rectange.height != 0`.
    #[expect(clippy::multiple_unsafe_ops_per_block)]
    #[expect(clippy::undocumented_unsafe_blocks)]
    pub unsafe fn fill_unchecked(&mut self, color: Color, rectangle: Rectangle) {
        let block_stride = {
            let block_pixel_stride = self.info.stride() - rectangle.width;

            block_pixel_stride * self.info.bytes_per_pixel()
        };

        // SAFETY:
        // - `rectangle.top_left.x` is less than `self.info().width()`.
        // - `rectangle.top_left.y` is less than `self.info().height()`.
        let (mut r, mut g, mut b) = unsafe { self.setup_rgb_pointers(rectangle.top_left) };

        for _ in 0..(rectangle.height - 1) {
            for _ in 0..(rectangle.width) {
                unsafe {
                    r.write_volatile(color.r);
                    g.write_volatile(color.g);
                    b.write_volatile(color.b);
                }
                unsafe {
                    r = r.add(self.info.bytes_per_pixel());
                    g = g.add(self.info.bytes_per_pixel());
                    b = b.add(self.info.bytes_per_pixel());
                };
            }

            unsafe {
                r = r.add(block_stride);
                g = g.add(block_stride);
                b = b.add(block_stride);
            }
        }

        for _ in 0..(rectangle.width - 1) {
            unsafe {
                r.write_volatile(color.r);
                g.write_volatile(color.g);
                b.write_volatile(color.b);
            }
            unsafe {
                r = r.add(self.info.bytes_per_pixel());
                g = g.add(self.info.bytes_per_pixel());
                b = b.add(self.info.bytes_per_pixel());
            };
        }

        unsafe {
            r.write_volatile(color.r);
            g.write_volatile(color.g);
            b.write_volatile(color.b);
        }
    }

    /// Returns three pointers set up to write a properly formatted pixel.
    ///
    /// # Safety
    // - `coords.x` is less than `self.info().width()`.
    // - `coords.y` is less than `self.info().height()`.
    #[allow(clippy::many_single_char_names)]
    unsafe fn setup_rgb_pointers(&self, coords: PixelCoordinates) -> (*mut u8, *mut u8, *mut u8) {
        debug_assert!(coords.x < self.info.width());
        debug_assert!(coords.y < self.info.height());

        let base_ptr = {
            let row_pixel_offset = self.info.stride() * coords.y;
            let pixel_offset = row_pixel_offset + coords.x;

            let byte_offset = pixel_offset * self.info.bytes_per_pixel();

            // SAFETY:
            // The invariants of [`Info`] mean that the invariants of the function
            // make this operation safe.
            unsafe { self.ptr.add(byte_offset) }
        };

        // SAFETY:
        // The invariants of [`Info`] mean that the invariants of the function
        // make this operation safe.
        let g = unsafe { base_ptr.add(1) };

        let (r, b) = match self.info.format() {
            PixelFormat::Rgb => {
                // SAFETY:
                // The invariants of [`Info`] mean that the invariants of the function
                // make this operation safe.
                let b = unsafe { base_ptr.add(2) };
                (base_ptr, b)
            }
            PixelFormat::Bgr => {
                // SAFETY:
                // The invariants of [`Info`] mean that the invariants of the function
                // make this operation safe.
                let r = unsafe { base_ptr.add(2) };
                (r, base_ptr)
            }
        };

        (r, g, b)
    }
}
