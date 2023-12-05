//! Implementations of the SHA-512 algorithm.

use core::{fmt::Display, mem::MaybeUninit};

use crate::decode_hex;

pub mod bits;
pub mod bytes;

/// Size of a SHA-512 digest in bytes.
pub const DIGEST_BYTES: usize = 64;

// Various internal constants.
/// The number of bits in each hash block for SHA-512.
const BLOCK_SIZE_BITS: u16 = 1024;
/// The number of bytes in each hash block for SHA-512.
const BLOCK_SIZE_BYTES: u8 = 128;
#[expect(
    clippy::unreadable_literal,
    reason = "round constants should never need to be read"
)]
/// The round constants for SHA-512.
const ROUND_CONSTANTS: [u64; 80] = [
    0x428a2f98d728ae22,
    0x7137449123ef65cd,
    0xb5c0fbcfec4d3b2f,
    0xe9b5dba58189dbbc,
    0x3956c25bf348b538,
    0x59f111f1b605d019,
    0x923f82a4af194f9b,
    0xab1c5ed5da6d8118,
    0xd807aa98a3030242,
    0x12835b0145706fbe,
    0x243185be4ee4b28c,
    0x550c7dc3d5ffb4e2,
    0x72be5d74f27b896f,
    0x80deb1fe3b1696b1,
    0x9bdc06a725c71235,
    0xc19bf174cf692694,
    0xe49b69c19ef14ad2,
    0xefbe4786384f25e3,
    0x0fc19dc68b8cd5b5,
    0x240ca1cc77ac9c65,
    0x2de92c6f592b0275,
    0x4a7484aa6ea6e483,
    0x5cb0a9dcbd41fbd4,
    0x76f988da831153b5,
    0x983e5152ee66dfab,
    0xa831c66d2db43210,
    0xb00327c898fb213f,
    0xbf597fc7beef0ee4,
    0xc6e00bf33da88fc2,
    0xd5a79147930aa725,
    0x06ca6351e003826f,
    0x142929670a0e6e70,
    0x27b70a8546d22ffc,
    0x2e1b21385c26c926,
    0x4d2c6dfc5ac42aed,
    0x53380d139d95b3df,
    0x650a73548baf63de,
    0x766a0abb3c77b2a8,
    0x81c2c92e47edaee6,
    0x92722c851482353b,
    0xa2bfe8a14cf10364,
    0xa81a664bbc423001,
    0xc24b8b70d0f89791,
    0xc76c51a30654be30,
    0xd192e819d6ef5218,
    0xd69906245565a910,
    0xf40e35855771202a,
    0x106aa07032bbd1b8,
    0x19a4c116b8d2d0c8,
    0x1e376c085141ab53,
    0x2748774cdf8eeb99,
    0x34b0bcb5e19b48a8,
    0x391c0cb3c5c95a63,
    0x4ed8aa4ae3418acb,
    0x5b9cca4f7763e373,
    0x682e6ff3d6b2b8a3,
    0x748f82ee5defb2fc,
    0x78a5636f43172f60,
    0x84c87814a1f0ab72,
    0x8cc702081a6439ec,
    0x90befffa23631e28,
    0xa4506cebde82bde9,
    0xbef9a3f7b2c67915,
    0xc67178f2e372532b,
    0xca273eceea26619c,
    0xd186b8c721c0c207,
    0xeada7dd6cde0eb1e,
    0xf57d4f7fee6ed178,
    0x06f067aa72176fba,
    0x0a637dc5a2c898a6,
    0x113f9804bef90dae,
    0x1b710b35131c471b,
    0x28db77f523047d84,
    0x32caab7b40c72493,
    0x3c9ebe0a15c9bebc,
    0x431d67c49c100d4c,
    0x4cc5d4becb3e42b6,
    0x597f299cfc657e2a,
    0x5fcb6fab3ad6faec,
    0x6c44198c4a475817,
];
#[expect(
    clippy::unreadable_literal,
    reason = "initial hash values should never need to be read"
)]
/// The initial hash state for SHA-512.
const INITIAL_HASH_VALUES: [u64; 8] = [
    0x6a09e667f3bcc908,
    0xbb67ae8584caa73b,
    0x3c6ef372fe94f82b,
    0xa54ff53a5f1d36f1,
    0x510e527fade682d1,
    0x9b05688c2b3e6c1f,
    0x1f83d9abfb41bd6b,
    0x5be0cd19137e2179,
];

/// Function for SHA-512.
const fn ch(x: u64, y: u64, z: u64) -> u64 {
    (x & y) ^ (!x & z)
}

/// Function for SHA-512.
const fn maj(x: u64, y: u64, z: u64) -> u64 {
    (x & y) ^ (x & z) ^ (y & z)
}

/// Function for SHA-512.
const fn csigma_0(x: u64) -> u64 {
    x.rotate_right(28) ^ x.rotate_right(34) ^ x.rotate_right(39)
}

/// Function for SHA-512.
const fn csigma_1(x: u64) -> u64 {
    x.rotate_right(14) ^ x.rotate_right(18) ^ x.rotate_right(41)
}

/// Function for SHA-512.
const fn lsigma_0(x: u64) -> u64 {
    x.rotate_right(1) ^ x.rotate_right(8) ^ (x >> 7)
}

/// Function for SHA-512.
const fn lsigma_1(x: u64) -> u64 {
    x.rotate_right(19) ^ x.rotate_right(61) ^ (x >> 6)
}

/// Hashs a block of SHA-512.
fn hash_block(hash_state: &mut [u64; 8], block: &[u8; 128]) {
    #![expect(
        clippy::many_single_char_names,
        reason = "SHA-512 algorithm uses the short variable names"
    )]

    let mut working_buffer: [u64; 80] = [0; 80];

    // SAFETY:
    // - `block` is valid for reads of 128 bytes, since it is a 128 byte slice
    // - `working_buffer` is valid for writes of 128 bytes, since it is 640 bytes
    // - `block` and `working_buffer` are properly aligned, since `u8` needs no alignment
    // - `block` comes from outside the function, whereas `working_buffer` is on the stack,
    //      thus they cannot overlap
    unsafe {
        core::ptr::copy_nonoverlapping(
            block.as_ptr(),
            working_buffer.as_mut_ptr().cast::<u8>(),
            128,
        );
    }

    for slot in working_buffer.iter_mut().take(16) {
        *slot = slot.to_be();
    }

    for index in 16..80 {
        working_buffer[index] = lsigma_1(working_buffer[index - 2])
            .wrapping_add(working_buffer[index - 7])
            .wrapping_add(lsigma_0(working_buffer[index - 15]))
            .wrapping_add(working_buffer[index - 16]);
    }

    let mut a = hash_state[0];
    let mut b = hash_state[1];
    let mut c = hash_state[2];
    let mut d = hash_state[3];
    let mut e = hash_state[4];
    let mut f = hash_state[5];
    let mut g = hash_state[6];
    let mut h = hash_state[7];

    for index in 0..80 {
        let temp1 = h
            .wrapping_add(csigma_1(e))
            .wrapping_add(ch(e, f, g))
            .wrapping_add(ROUND_CONSTANTS[index])
            .wrapping_add(working_buffer[index]);
        let temp2 = csigma_0(a).wrapping_add(maj(a, b, c));

        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(temp1);
        d = c;
        c = b;
        b = a;
        a = temp1.wrapping_add(temp2);
    }

    hash_state[0] = a.wrapping_add(hash_state[0]);
    hash_state[1] = b.wrapping_add(hash_state[1]);
    hash_state[2] = c.wrapping_add(hash_state[2]);
    hash_state[3] = d.wrapping_add(hash_state[3]);
    hash_state[4] = e.wrapping_add(hash_state[4]);
    hash_state[5] = f.wrapping_add(hash_state[5]);
    hash_state[6] = g.wrapping_add(hash_state[6]);
    hash_state[7] = h.wrapping_add(hash_state[7]);
}

/// A representation of a SHA-512 digest.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct Digest {
    /// Stored in 8 64-bit native-endian integers.
    digest: [u64; DIGEST_BYTES / 8],
}

impl Digest {
    /// Constructs a [`Digest`] from `bytes`.
    #[must_use]
    pub const fn from_bytes(bytes: &[u8]) -> Option<Digest> {
        if bytes.len() != 64 {
            return None;
        }

        Some(Self::from_bytes_infallible(&bytes.as_chunks().0[0]))
    }

    /// Constructs a [`Digest`] from its representation as a byte array.
    ///
    /// When starting from a slice rather than an array, [`from_bytes()`] can be used
    #[must_use]
    pub const fn from_bytes_infallible(bytes: &[u8; DIGEST_BYTES]) -> Digest {
        let mut digest = Digest {
            digest: [0; DIGEST_BYTES / 8],
        };

        let windows = bytes.as_chunks::<8>().0;

        digest.digest[0] = u64::from_be_bytes(windows[0]);
        digest.digest[1] = u64::from_be_bytes(windows[1]);
        digest.digest[2] = u64::from_be_bytes(windows[2]);
        digest.digest[3] = u64::from_be_bytes(windows[3]);
        digest.digest[4] = u64::from_be_bytes(windows[4]);
        digest.digest[5] = u64::from_be_bytes(windows[5]);
        digest.digest[6] = u64::from_be_bytes(windows[6]);
        digest.digest[7] = u64::from_be_bytes(windows[7]);

        digest
    }

    /// Constructs a [`Digest`] from its representation as a hex string.
    #[must_use]
    pub const fn from_str(hash: &str) -> Option<Digest> {
        if hash.len() != 128 {
            return None;
        }

        let mut output = [MaybeUninit::uninit(); DIGEST_BYTES];

        let output = if decode_hex(hash.as_bytes(), &mut output) {
            // SAFETY:
            // `decode_hex` initializes `hash.as_bytes() / 2`, which equals 64.
            unsafe { MaybeUninit::array_assume_init(output) }
        } else {
            return None;
        };

        Some(Self::from_bytes_infallible(&output))
    }

    /// Constructs a [`Digest`] from its representation as a hex string produced by `iter`.
    pub fn from_chars<I: Iterator<Item = char>>(iter: I) -> Option<Digest> {
        let mut chars = [0; 128];

        let mut remaining: &mut [u8] = &mut chars;

        for ch in iter {
            if ch.len_utf8() > remaining.len() {
                return None;
            }

            ch.encode_utf8(remaining);

            remaining = &mut remaining[ch.len_utf8()..];
        }

        let created = core::str::from_utf8(&chars).ok()?;

        Self::from_str(created)
    }

    /// Takes a native-endian representation of a SHA512 digest.
    #[must_use]
    pub const fn from_u64s(hash: [u64; 8]) -> Digest {
        Digest { digest: hash }
    }

    /// Returns a native-endian representation of a SHA512 digest.
    #[must_use]
    pub const fn as_u64s(&self) -> [u64; DIGEST_BYTES / 8] {
        self.digest
    }
}

/// Returned when the number of bits hashed by a SHA-512 implementation
/// hashs `2^128 - 1023` bits.
///
/// On any reasonably sized input, should never happen.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct UpdateBitsError;

impl Display for UpdateBitsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "SHA-512 can only hash 2^128 - 1023 bits")
    }
}

#[cfg(test)]
mod test {
    use super::{Digest, DIGEST_BYTES};

    const DIGEST_1: Digest = Digest::from_u64s([
        0x0340_f948_f096_c1cb,
        0xd1b7_3efd_a6f5_4a49,
        0xc28b_8bd9_397a_ba28,
        0x5abf_8552_293e_dde6,
        0x4fc3_763a_0db9_cad7,
        0x53bd_1e19_03a7_6d65,
        0x18de_de79_36ae_3896,
        0xe699_4aad_8db6_4eb3,
    ]);

    const DIGEST_1_BYTES: [u8; DIGEST_BYTES] = [
        0x03, 0x40, 0xf9, 0x48, 0xf0, 0x96, 0xc1, 0xcb, // 1st u64
        0xd1, 0xb7, 0x3e, 0xfd, 0xa6, 0xf5, 0x4a, 0x49, // 2nd u64
        0xc2, 0x8b, 0x8b, 0xd9, 0x39, 0x7a, 0xba, 0x28, // 3rd u64
        0x5a, 0xbf, 0x85, 0x52, 0x29, 0x3e, 0xdd, 0xe6, // 4th u64
        0x4f, 0xc3, 0x76, 0x3a, 0x0d, 0xb9, 0xca, 0xd7, // 5th u64
        0x53, 0xbd, 0x1e, 0x19, 0x03, 0xa7, 0x6d, 0x65, // 6th u64
        0x18, 0xde, 0xde, 0x79, 0x36, 0xae, 0x38, 0x96, // 7th u64
        0xe6, 0x99, 0x4a, 0xad, 0x8d, 0xb6, 0x4e, 0xb3, // 8th u64
    ];

    const DIGEST_1_STR: &str = "0340f948f096c1cbd1b73efda6f54a49c28b8bd9397aba285abf8552293edde64fc3763a0db9cad753bd1e1903a76d6518dede7936ae3896e6994aad8db64eb3";

    #[test]
    #[should_panic]
    fn from_bytes_too_long() {
        Digest::from_bytes(&[0; 129]).unwrap();
    }

    #[test]
    fn from_bytes_1() {
        assert_eq!(Digest::from_bytes(&DIGEST_1_BYTES).unwrap(), DIGEST_1);
    }

    #[test]
    fn from_bytes_infallible_1() {
        assert_eq!(Digest::from_bytes_infallible(&DIGEST_1_BYTES), DIGEST_1);
    }

    #[test]
    fn from_str() {
        assert_eq!(Digest::from_str(DIGEST_1_STR).unwrap(), DIGEST_1);
    }

    #[test]
    fn from_chars() {
        assert_eq!(Digest::from_chars(DIGEST_1_STR.chars()).unwrap(), DIGEST_1)
    }
}
