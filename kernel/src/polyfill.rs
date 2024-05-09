//! Polyfills for various unstable APIs.

use core::mem::MaybeUninit;

/// Assuming all the elements are initialized, get a slice to them.
///
/// # Safety
///
/// It is up to the caller to guarantee that the [`MaybeUninit<T>`] elements really are in
/// an initialized state. Calling this when the content is not yet fully initialized causes
/// undefined behavior.
///
/// See [`MaybeUninit::assume_init_ref`] for more details and examples.
pub unsafe fn maybe_uninit_slice_assume_init_ref<T>(slice: &[MaybeUninit<T>]) -> &[T] {
    // SAFETY:
    // - Since `slice` is a valid slice, we know that `slice` is valid for reads of up to
    // `slice.len() * core::mem::size_of::<T>()` bytes and is properly aligned.
    // - According to the invariants of this function, `slice` points to `slice.len()` consecutive
    // initialized values of type `T`.
    // - The immutablilty of the memory referenced by `slice` will not be changed during
    // its lifetime without unsafe code.
    // - The total size of the slice does not change.
    unsafe { core::slice::from_raw_parts(slice.as_ptr().cast::<T>(), slice.len()) }
}
