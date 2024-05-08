//! Code for controlled modifications, placing all unsafety on the initialization function.

use core::cell::UnsafeCell;

/// Wrapper struct for variables that are modified by the loading process and never touched again.
pub struct ControllledModificationCell<T> {
    /// The value that is modified at load time.
    value: UnsafeCell<T>,
}

// SAFETY:
unsafe impl<T> Sync for ControllledModificationCell<T> where for<'a> &'a T: Send {}
// SAFETY:
unsafe impl<T> Send for ControllledModificationCell<T> where T: Sync {}

impl<T> ControllledModificationCell<T> {
    /// Constructs a new instance of [`LoadTimeMutable`] which will wrap `value`.
    pub const fn new(value: T) -> ControllledModificationCell<T> {
        ControllledModificationCell {
            value: UnsafeCell::new(value),
        }
    }

    /// Gets a reference to the contained value.
    #[allow(clippy::missing_panics_doc)]
    pub fn get(&self) -> &T {
        // SAFETY:
        // This item is only modified at load time, and so while this program is running, no changes may be observed.
        unsafe { self.value.get().as_ref().unwrap() }
    }

    /// Returns a mutable reference to the wrapped value.
    ///
    /// # Safety
    /// - The lifetime of the mutable reference produced by this function must not overlap
    /// with the lifetime of any other reference, mutable or not, pointing to this value.
    /// - All synchronization necessary to soundly mutate this value must be performed outside
    /// of this function.
    #[allow(clippy::mut_from_ref, clippy::missing_panics_doc)]
    pub unsafe fn get_mut(&self) -> &mut T {
        // SAFETY:
        // According to the invariants of this function, this is safe to call.
        unsafe { self.value.get().as_mut().unwrap() }
    }
}

impl<T: Copy> ControllledModificationCell<T> {
    /// Copies the stored value.
    pub fn copy(&self) -> T {
        // SAFETY:
        // This item is only modified at load time, and so while this program is running, no changes may be observed.
        unsafe { *self.value.get() }
    }
}
