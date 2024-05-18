//! Definitions and helper functions for UEFI datatypes.

/// A collection of related interfaces provided by UEFI firmware.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct RawHandle(pub *mut core::ffi::c_void);

/// A status code returned by UEFI functions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Status(usize);

impl Status {
    // Success codes.

    /// The operation completed successfully.
    pub const SUCCESS: Status = Status(0);

    // Warning codes.

    /// The string cotained one or more characters that the device could not render
    /// and were skipped.
    pub const WARN_UNKNOWN_GLYPH: Status = Status(1);    
    /// The handle was closed, but the file was not deleted.
    pub const WARN_DELETE_FAILURE: Status = Status(2); 
    /// The handle was closed, but the data to the file was not flushed properly.
    pub const WARN_WRITE_FAILURE: Status = Status(3); 
    /// The resulting buffer was too small, and the data was truncated to the buffer
    /// size.
    pub const WARN_BUFFER_TOO_SMALL: Status = Status(4);
    /// The data has not been updated within the timeframe set by local policy for this
    /// type of data.
    pub const WARN_STALE_DATA: Status = Status(5);
    /// The resulting buffer contains a UEFI-compliant file system.
    pub const WARN_FILE_SYSTEM: Status = Status(6); 
    /// The operation will be processed across a system reset.
    pub const WARN_RESET_REQUIRED: Status = Status(1); 

    // Error codes

    /// The image failed to load.
    pub const LOAD_ERROR: Status = Status(Status::ERROR_BIT | 1);
    /// A parameter was incorrect.
    pub const INVALID_PARAMETER: Status = Status(Status::ERROR_BIT | 2);
    /// The operation is not supported.
    pub const UNSUPPORTED: Status = Status(Status::ERROR_BIT | 3);
    /// The buffer was not the proper size for the request.
    pub const BAD_BUFFER_SIZE: Status = Status(Status::ERROR_BIT | 4);
    /// The buffer is not large enough to hold the requested data. The
    /// required buffer size is returned in the appropriate parameter
    /// when this error occurs.
    pub const BUFFER_TOO_SMALL: Status = Status(Status::ERROR_BIT | 5);
    /// There is no pending data upon return.
    pub const NOT_READY: Status = Status(Status::ERROR_BIT | 6);
    /// The physical device reported an error while attempting the operation.
    pub const DEVICE_ERROR: Status = Status(Status::ERROR_BIT | 7);
    /// The device cannot be written to.
    pub const WRITE_PROTECTED: Status = Status(Status::ERROR_BIT | 8);
    /// A resource has run out.
    pub const OUT_OF_RESOURCES: Status = Status(Status::ERROR_BIT | 9);
    /// An inconsistency was detected on the file system causing the operation
    /// to fail.
    pub const VOLUME_CORRUPTED: Status = Status(Status::ERROR_BIT | 10);
    /// There is no more space on the file system.
    pub const VOLUME_FULL: Status = Status(Status::ERROR_BIT | 11);
    /// The device does not contain any medium to perform the operation.
    pub const NO_MEDIA: Status = Status(Status::ERROR_BIT | 12);
    /// The medium in the device has changed since the last access.
    pub const MEDIA_CHANGED: Status = Status(Status::ERROR_BIT | 13);
    /// The item was not found.
    pub const NOT_FOUND: Status = Status(Status::ERROR_BIT | 14);
    /// Access was denied.
    pub const ACCESS_DENIED: Status = Status(Status::ERROR_BIT | 15);
    /// The server was not found or did not respond to the request.
    pub const NO_RESPONSE: Status = Status(Status::ERROR_BIT | 16);
    /// A mapping to a device does not exist.
    pub const NO_MAPPING: Status = Status(Status::ERROR_BIT | 17);
    /// The timeout time expired.
    pub const TIMEOUT: Status = Status(Status::ERROR_BIT | 18);
    /// The protocol has not been started.
    pub const NOT_STARTED: Status = Status(Status::ERROR_BIT | 19);
    /// The protocol has already been started.
    pub const ALREADY_STARTED: Status = Status(Status::ERROR_BIT | 20);
    /// The operation was aborted.
    pub const ABORTED: Status = Status(Status::ERROR_BIT | 21);
    /// An ICMP error occurred during the network operation.
    pub const ICMP_ERROR: Status = Status(Status::ERROR_BIT | 22);
    /// A TFTP error occurred during the network operation.
    pub const TFTP_ERROR: Status = Status(Status::ERROR_BIT | 23);
    /// A protocol error occurred during the network operation.
    pub const PROTOCOL_ERROR: Status = Status(Status::ERROR_BIT | 24);
    /// The operation encountered an internal version that was incompatible
    /// with the version requested by the caller.
    pub const INCOMPATIBLE_VERSION: Status = Status(Status::ERROR_BIT | 25);
    /// The operation was not performed due to a security violation.
    pub const SECURITY_VIOLATION: Status = Status(Status::ERROR_BIT | 26);
    /// A CRC error was detected.
    pub const CRC_ERROR: Status = Status(Status::ERROR_BIT | 27);
    /// Beginning or end of media was reached.
    pub const END_OF_MEDIA: Status = Status(Status::ERROR_BIT | 28);
    /// The end of the file was reached.
    pub const END_OF_FILE: Status = Status(Status::ERROR_BIT | 31);
    /// The language specified was invalid.
    pub const INVALID_LANGUAGE: Status = Status(Status::ERROR_BIT | 32);
    /// The security status of the data is unknown or compromised and the data
    /// must be updated or replaced to restore a valid security status.
    pub const COMPROMISED_DATA: Status = Status(Status::ERROR_BIT | 33);
    /// There is an address conflict during address allocation.
    pub const IP_ADDRESS_CONFLICT: Status = Status(Status::ERROR_BIT | 34);
    /// A HTTP error occurred during the network operation.
    pub const HTTP_ERROR: Status = Status(Status::ERROR_BIT | 35);

    /// All [`Status`]'s with the [`Status::ERROR_BIT`] set are error codes.
    const ERROR_BIT: usize = 1 << (usize::BITS - 1);
    /// All [`Status`]'s with the [`Status::OEM_BIT`] set are reserved for use
    /// by OEMs.
    const OEM_BIT: usize = 1 << (usize::BITS - 2);

    /// Returns `true` if `self` is an warning code.
    pub fn warning(self) -> bool {
        self.0 & Status::ERROR_BIT == Status::ERROR_BIT && self != Status::SUCCESS
    }

    /// Returns `true` if `self` is an error code.
    pub fn error(self) -> bool {
        self.0 & Status::ERROR_BIT == Status::ERROR_BIT
    }

    /// Returns `true` is `self` is reserved for use by UEFI, otherwise returns true.
    pub const fn uefi(self) -> bool {
        self.0 & Status::OEM_BIT == 0
    }

    /// Returns `true` if `self` is reserved for use by OEMs, otherwise returns `false`.
    pub const fn oem(self) -> bool {
        self.0 & Status::OEM_BIT == Status::OEM_BIT
    }
}

/// A unicode codepoint in UCS-2 encoding.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Char16(u16);

impl Char16 {
    /// The NUL character in UCS-2 encoding.
    pub const NUL: Char16 = Char16(0);

    /// Converts `c` to a [`Char16`], returning `None` if `c` is not a valid
    /// UCS-2 character.
    pub const fn new(c: char) -> Option<Char16> {
        let codepoint = c as u32;
        if codepoint <= 0xFFFF {
            #[allow(clippy::cast_possible_truncation)]
            Some(Char16(codepoint as u16))
        } else {
            None
        }
    }

    /// Converts `value` to a [`Char16`], returning `None` if `value` is not a valid
    /// UCS-2 character.
    pub const fn from_u32(value: u32) -> Option<Char16> {
        match char::from_u32(value) {
            Some(c) => Char16::new(c),
            None => None,
        }
    }

    /// Converts `value` to a [`Char16`], returning `None` if `value` is not a valid
    /// UCS-2 character.
    pub const fn from_u16(value: u16) -> Option<Char16> {
        match char::from_u32(value as u32) {
            Some(c) => Char16::new(c),
            None => None,
        }
    }

    /// Returns the [`char`] the [`Char16`] represents.
    pub const fn to_char(self) -> char {
        // SAFETY:
        // All valid UCS-2 characters are valid [`char`]s.
        unsafe { core::mem::transmute::<u32, char>(self.0 as u32) }
    }
}

/// A UCS-2 encoded null-terminated string.
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct CStr16([Char16]);

impl CStr16 {
    /// Converts `ptr` into a safe [`CStr16`] wrapper.
    ///
    /// # Safety
    /// - `ptr` must point to a properly aligned region of memory that
    ///     that contains a [`Char16::NUL`] character.
    /// - The region of memory to which `ptr` points must be valid [`Char16`]s until the [`Char16::NUL`].
    #[must_use]
    pub const unsafe fn from_ptr<'ptr>(mut ptr: *const Char16) -> &'ptr Self {
        let mut length = 0;
        // SAFETY:
        // A valid [`CStr16`] wrapper must point to at least one [`Char16`].
        let mut c = unsafe { *ptr };
        while c.to_char() as u16 != Char16::NUL.to_char() as u16 {
            length += 1;
            // SAFETY:
            // `ptr` points to a region that contains a [`Char16::NUL`] character,
            // which we have not encountered, so `ptr + 1` must be in bounds.
            ptr = unsafe { ptr.add(1) };
            // SAFETY:
            // We have not encounted a [`Char16::NUL`], so reading the next character
            // must be safe.
            c = unsafe { *ptr };
        }

        // SAFETY:
        // `ptr` = `start_ptr` + `length`, so `ptr` - `length` must be safe.
        ptr = unsafe { ptr.sub(length) };

        // SAFETY:
        // - `ptr` is valid for reads up to and including the [`Char16::NUL`].
        let slice = unsafe { core::slice::from_raw_parts(ptr.cast::<u16>(), length + 1) };

        // SAFETY:
        // - By the invariants of this function, `ptr` points to valid [`Char16`]s.
        // - The slice ends with a [`Char16::NUL`] due to the loop above.
        // - We end on the first [`Char16::NUL`], so `slice` can't contain interior [`Char16::NUL`]s.
        unsafe { Self::from_u16_with_nul_unchecked(slice) }
    }

    /// Converts `slice` into a safe [`CStr16`] wrapper.
    ///
    /// # Panics
    /// If an interior [`Char16::NUL`] is found or if `slice` does not end
    /// with a [`Char16::NUL`].
    pub const fn from_slice(slice: &[Char16]) -> &Self {
        let mut index = 0;

        while index < slice.len() - 1 {
            assert!(slice[index].to_char() as u16 != Char16::NUL.to_char() as u16);
            index += 1;
        }

        assert!(slice[index].to_char() as u16 == Char16::NUL.to_char() as u16);

        // SAFETY:
        // `slice` is a valid [`Char16`] arr to underly a [`CStr16`].
        unsafe { &*(core::ptr::from_ref(slice) as *const Self) }
    }

    /// Unsafely creates a [`CStr16`] from a [`u16`] slice.
    ///
    /// # Safety
    /// - The [`u16`]s that make up `str` must be valid [`Char16`]s.
    /// - `str[str.len() - 1]` must be [`Char16::NUL`].
    /// - `str` must not contain interior [`Char16::NUL`]s.
    #[must_use]
    pub const unsafe fn from_u16_with_nul_unchecked(str: &[u16]) -> &Self {
        // SAFETY:
        // The slice satisfies the invariants of this structure.
        unsafe { &*(core::ptr::from_ref(str) as *const Self) }
    }

    /// Returns a pointer to this [`CStr16`].
    #[must_use]
    pub const fn as_ptr(&self) -> *const Char16 {
        self.0.as_ptr()
    }

    /// Returns the underlying [`Char16`] slice without the trailing [`Char16::NUL`].
    pub fn as_slice(&self) -> &[Char16] {
        &self.0[..self.0.len() - 1]
    }
}

#[macro_export]
macro_rules! cstr16 {
    ($str: tt) => {{
        const STR: &str = $str;

        const CHAR_COUNT: usize = {
            let str_bytes = STR.as_bytes();

            let mut byte_index = 0;
            let mut count = 1;

            while byte_index < str_bytes.len() {
                if str_bytes[byte_index] & 0b1000_0000 == 0 {
                    byte_index += 1;
                    count += 1;
                } else if str_bytes[byte_index] & 0b1110_0000 == 0b1100_0000 {
                    byte_index += 2;
                    count += 2;
                } else if str_bytes[byte_index] & 0b1111_0000 == 0b1110_0000 {
                    byte_index += 3;
                    count += 3;
                } else {
                    panic!("Unsupported codepoint");
                }
            }

            count
        };

        const ARR: [$crate::datatypes::Char16; CHAR_COUNT] = {
            let str_bytes = STR.as_bytes();
            let mut output = [$crate::datatypes::Char16::NUL; CHAR_COUNT];

            let mut byte_index = 0;
            let mut index = 0;

            while byte_index < str_bytes.len() {
                let value = if str_bytes[byte_index] & 0b1000_0000 == 0 {
                    let value = str_bytes[byte_index] as u16 & !0b1000_0000;
                    byte_index += 1;

                    value
                } else if str_bytes[byte_index] & 0b1110_0000 == 0b1100_0000 {
                    let value = ((str_bytes[byte_index] as u16 & !0b1000_0000) << 6)
                        | (str_bytes[byte_index + 1] as u16 & !0b1100_0000);

                    byte_index += 2;

                    value
                } else if str_bytes[byte_index] & 0b1111_0000 == 0b1110_0000 {
                    let value = ((str_bytes[byte_index] as u16 & !0b1000_0000) << 12)
                        | ((str_bytes[byte_index + 1] as u16 & !0b1100_0000) << 6)
                        | (str_bytes[byte_index + 2] as u16 & !0b1100_0000);

                    byte_index += 3;

                    value
                } else {
                    panic!("Unsupported codepoint");
                };

                if let Some(c) = $crate::datatypes::Char16::from_u16(value) {
                    if c.to_char() as u16 == $crate::datatypes::Char16::NUL.to_char() as u16 {
                        panic!("Interior NULs are not allowed");
                    }
                    output[index] = c;
                    index += 1;
                } else {
                    panic!("Unsupported codepoint");
                }
            }

            output
        };

        const REF: &$crate::datatypes::CStr16 = $crate::datatypes::CStr16::from_slice(&ARR);

        REF
    }};
}

#[cfg(test)]
mod tests {
    use crate::datatypes::Char16;

    #[test]
    fn cstr16_macro() {
        let k = cstr16!("Hello");

        assert_eq!(k.as_slice()[0], Char16::new('H').unwrap());
        assert_eq!(k.as_slice()[1], Char16::new('e').unwrap());
        assert_eq!(k.as_slice()[2], Char16::new('l').unwrap());
        assert_eq!(k.as_slice()[3], Char16::new('l').unwrap());
        assert_eq!(k.as_slice()[4], Char16::new('o').unwrap());
    }
}
