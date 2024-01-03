//! Implementation of a bit-based SHA-512 hasher.

use core::{mem, ptr};

use super::{
    hash_block, Digest, UpdateBitsError, BLOCK_SIZE_BITS, BLOCK_SIZE_BYTES, INITIAL_HASH_VALUES,
};

/// A bit-based SHA-512 hasher.
pub struct Sha512 {
    /// Potential remainder from previous hashing operation.
    remainder: [u8; BLOCK_SIZE_BYTES as usize],
    /// The number of remaining bits from previous hashing operations.
    ///
    /// Will always be less than 1024.
    remainder_count: u16,

    /// Total number of bits hashed and in the remainder.
    total_bitcount: u128,

    /// The current state of the hash.
    hash_state: [u64; 8],
}

impl Sha512 {
    /// Constructs a new `Sha512` in its default state.
    #[must_use]
    pub fn new() -> Sha512 {
        Sha512 {
            remainder: [0; BLOCK_SIZE_BYTES as usize],
            remainder_count: 0,
            total_bitcount: 0,
            hash_state: INITIAL_HASH_VALUES,
        }
    }

    /// A convenience function that returns the [`Sha512`] [`Digest`] of a single `message`.
    pub fn hash(message: &[u8], bit_count: u128) -> Result<Digest, UpdateBitsError> {
        /// A static assertion that the message cannot overflow the bit counter.
        const ASSERTION: usize = (usize::BITS < 128) as usize - 1;

        core::hint::black_box(ASSERTION);

        let mut hasher = Sha512::default();

        hasher.update(message, bit_count)?;

        Ok(hasher.finalize())
    }

    /// Adds `bit_count` bits from `message` to the hash.
    ///
    /// Does not check if the bit representation is canonical.
    ///  
    /// # Errors
    /// Returns an error if the total number of bits hashed by the function would overflow a u128.
    pub fn update(&mut self, message: &[u8], mut bit_count: u128) -> Result<(), UpdateBitsError> {
        if bit_count == 0 {
            return Ok(());
        }

        if !self
            .total_bitcount
            .checked_add(bit_count)
            .is_some_and(|new_bitcount| {
                new_bitcount <= u128::MAX - (u128::from(BLOCK_SIZE_BITS) - 1)
            })
        {
            return Err(UpdateBitsError);
        }

        self.total_bitcount += bit_count;

        let stored_bits = self.remainder_count & 0b111;
        let needed_bits = 8 - stored_bits as u8;

        let storage_mask = !((needed_bits & 8) ^ 8).wrapping_sub(1);

        let mut storage = {
            let start_byte = (self.remainder_count >> 3) as usize;

            self.remainder_count -= stored_bits;

            self.remainder[start_byte] & storage_mask
        };

        let mut byte_offset = 0;

        while bit_count > 8 {
            let current_write = &mut self.remainder[(self.remainder_count >> 3) as usize];

            *current_write = storage | message[byte_offset] >> stored_bits;
            storage = (message[byte_offset] << (needed_bits & 0b111)) & storage_mask;

            self.remainder_count += 8;
            bit_count -= 8;
            byte_offset += 1;

            if self.remainder_count == BLOCK_SIZE_BITS {
                hash_block(&mut self.hash_state, &self.remainder);
                self.remainder_count = 0;
            }
        }

        let current_write = &mut self.remainder[(self.remainder_count >> 3) as usize];
        *current_write = storage | message[byte_offset] >> stored_bits;

        #[allow(
            clippy::cast_possible_truncation,
            reason = "`bit_count` should always be less than or equal to 8 due to the above while loop"
        )]
        {
            self.remainder_count += stored_bits + bit_count as u16;
        }

        if self.remainder_count == BLOCK_SIZE_BITS {
            hash_block(&mut self.hash_state, &self.remainder);
            self.remainder_count = 0;
        }

        Ok(())
    }

    /// Returns the final hash of all bits passed into the hasher.
    ///
    /// Resets the hasher to it's initial state.
    pub fn finalize(&mut self) -> Digest {
        let one_byte = usize::from(self.remainder_count >> 3);
        let one_bit = self.remainder_count & 0b111;

        // Set the 1 bit according to the algorithm, and zero any bits after the 1 bit.
        let bit: u8 = 1 << (7 - one_bit);

        self.remainder[one_byte] = (self.remainder[one_byte] | bit) & !(bit - 1);

        let mut first_zero_byte = one_byte + 1;

        let minumum_remaining = self.remainder_count + 1 + 128;

        if minumum_remaining > BLOCK_SIZE_BITS {
            self.remainder[first_zero_byte..].fill(0);

            hash_block(&mut self.hash_state, &self.remainder);

            first_zero_byte = 0;
        }

        self.remainder[first_zero_byte..].fill(0);

        // Get the space for the 128 bit bit count.
        let bitcount_bytes =
            &mut self.remainder[BLOCK_SIZE_BYTES as usize - 16..BLOCK_SIZE_BYTES as usize];

        #[cfg(target_endian = "little")]
        let bit_count = u128::from(((self.total_bitcount >> 64u64) as u64).to_be())
            | (u128::from(((self.total_bitcount & 0xFFFF_FFFF_FFFF_FFFF) as u64).swap_bytes())
                << 64);

        #[cfg(target_endian = "big")]
        let bit_count = self.total_bitcount;

        // SAFETY:
        // - [`u8`] has no alignment constraints, so all pointers are properly aligned
        // - `bit_count` is an initialized [`u128`], and so is valid for reads of 16 bytes
        // - `bitcount_bytes` is an initialized 16 byte slice, and so is valid for reads
        // - `bitcount_bytes` and `bit_count` distinct items on the stack
        unsafe {
            core::ptr::copy_nonoverlapping(
                ptr::addr_of!(bit_count).cast::<u8>(),
                bitcount_bytes.as_mut_ptr(),
                mem::size_of::<u128>(),
            );
        }

        hash_block(&mut self.hash_state, &self.remainder);

        let final_hash = self.hash_state;

        *self = Self::new();

        Digest { digest: final_hash }
    }
}

impl Default for Sha512 {
    fn default() -> Self {
        Sha512::new()
    }
}

#[cfg(all(test, not(any(target_os = "uefi", target_os = "none"))))]
mod test {
    #![expect(
        clippy::unreadable_literal,
        reason = "the long hex constants should never need to be read"
    )]

    use crate::sha512::Digest;

    use super::Sha512;

    #[test]
    fn abc() {
        const RESULT: Digest = Digest::from_u64s([
            0xddaf35a193617aba,
            0xcc417349ae204131,
            0x12e6fa4e89a97ea2,
            0x0a9eeee64b55d39a,
            0x2192992a274fc1a8,
            0x36ba3c23a3feebbd,
            0x454d4423643ce80e,
            0x2a9ac94fa54ca49f,
        ]);

        let mut sha512 = Sha512::new();

        let msg = b"abc";

        sha512.update(msg, msg.len() as u128 * 8).unwrap();

        let hash = sha512.finalize();

        assert_eq!(hash, RESULT);
    }

    #[test]
    fn blank() {
        const RESULT: Digest = Digest::from_u64s([
            0xcf83e1357eefb8bd,
            0xf1542850d66d8007,
            0xd620e4050b5715dc,
            0x83f4a921d36ce9ce,
            0x47d0d13c5d85f2b0,
            0xff8318d2877eec2f,
            0x63b931bd47417a81,
            0xa538327af927da3e,
        ]);

        let mut sha512 = Sha512::new();

        let msg = b"";

        sha512.update(msg, msg.len() as u128 * 8).unwrap();

        let hash = sha512.finalize();

        assert_eq!(hash, RESULT);
    }

    #[test]
    fn case_0() {
        const RESULT: Digest = Digest::from_u64s([
            0x204a8fc6dda82f0a,
            0x0ced7beb8e08a416,
            0x57c16ef468b228a8,
            0x279be331a703c335,
            0x96fd15c13b1b07f9,
            0xaa1d3bea57789ca0,
            0x31ad85c7a71dd703,
            0x54ec631238ca3445,
        ]);

        let mut sha512 = Sha512::new();

        let msg = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";

        sha512.update(msg, msg.len() as u128 * 8).unwrap();

        let hash = sha512.finalize();

        assert_eq!(hash, RESULT);
    }

    #[test]
    fn case_1() {
        const RESULT: Digest = Digest::from_u64s([
            0x8e959b75dae313da,
            0x8cf4f72814fc143f,
            0x8f7779c6eb9f7fa1,
            0x7299aeadb6889018,
            0x501d289e4900f7e4,
            0x331b99dec4b5433a,
            0xc7d329eeb6dd2654,
            0x5e96e55b874be909,
        ]);

        let mut sha512 = Sha512::new();

        let msg = b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu";

        sha512.update(msg, msg.len() as u128 * 8).unwrap();

        let hash = sha512.finalize();

        assert_eq!(hash, RESULT);
    }

    #[test]
    #[expect(
        clippy::large_stack_arrays,
        reason = "massive stack usage doesn't matter for short unit tests"
    )]
    fn case_2() {
        const RESULT: Digest = Digest::from_u64s([
            0xe718483d0ce76964,
            0x4e2e42c7bc15b463,
            0x8e1f98b13b204428,
            0x5632a803afa973eb,
            0xde0ff244877ea60a,
            0x4cb0432ce577c31b,
            0xeb009c5c2c49aa2e,
            0x4eadb217ad8cc09b,
        ]);

        let mut sha512 = Sha512::new();

        let msg = [b'a'; 1000000];

        sha512.update(&msg, msg.len() as u128 * 8).unwrap();

        let hash = sha512.finalize();

        assert_eq!(hash, RESULT);
    }

    #[test]
    fn case_3() {
        const RESULT: Digest = Digest::from_u64s([
            0xb47c933421ea2db1,
            0x49ad6e10fce6c7f9,
            0x3d0752380180ffd7,
            0xf4629a712134831d,
            0x77be6091b819ed35,
            0x2c2967a2e2d4fa50,
            0x50723c9630691f1a,
            0x05a7281dbe6c1086,
        ]);

        let mut sha512 = Sha512::new();

        let msg = b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmno";

        for _ in 0..16777216 {
            sha512.update(msg, msg.len() as u128 * 8).unwrap();
        }

        let hash = sha512.finalize();

        assert_eq!(hash, RESULT);
    }
}
