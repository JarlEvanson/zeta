//! Useful functions that don't belong in a more specific category.

/// Converts a [`u64`] to a [`usize`] without truncating.
#[cfg(target_pointer_width = "64")]
pub fn u64_to_usize(val: u64) -> usize {
    val as usize
}

/// Converts a [`u32`] to a [`usize`] without truncating.
#[cfg(any(target_pointer_width = "64", target_pointer_width = "32",))]
pub fn u32_to_usize(val: u32) -> usize {
    val as usize
}

/// Converts a [`u16`] to a [`usize`] without truncating.
#[cfg(any(
    target_pointer_width = "64",
    target_pointer_width = "32",
    target_pointer_width = "16"
))]
pub fn u16_to_usize(val: u16) -> usize {
    val as usize
}
