//! A simple [`SpinLock`] for the kernel.

use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

/// A mutual exclusion primitive useful to protecting shared data.
///
/// This mutex will spin waiting for the lock to become available.
pub struct SpinLock<T: ?Sized> {
    /// The lock.
    lock: AtomicBool,
    /// The value protected by the [`SpinLock`].
    value: UnsafeCell<T>,
}

// SAFETY:
// Nothing about `SpinLock<T>` changes whether it
// is safe to send `T` across threads.
unsafe impl<T: ?Sized + Send> Send for SpinLock<T> {}

// SAFETY:
// If `T` is safe to send across threads, then `SpinLock<T>`
// makes it safe to access from multiple threads simultaneously.
unsafe impl<T: ?Sized + Send> Sync for SpinLock<T> {}

impl<T> SpinLock<T> {
    /// Creates a new [`SpinLock`] in an unlocked state ready for use.
    pub const fn new(value: T) -> SpinLock<T> {
        SpinLock {
            lock: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }

    /// Consumes this mutex, returning the underlying data.
    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }
}

impl<T: ?Sized> SpinLock<T> {
    /// Acquires a mutex, spinning until it is able to do so.
    ///
    /// This function will spin until it is available to acquire the mutex. Upon returning, the context is the
    /// only context with the lock held. A RAII guard is returned to allow scoped unlock of the lock.
    pub fn lock(&self) -> SpinLockGuard<T> {
        loop {
            let was_locked = self.lock.load(Ordering::Relaxed);
            if !was_locked
                && self
                    .lock
                    .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                    .is_ok()
            {
                break;
            }

            core::hint::spin_loop();
        }

        SpinLockGuard { mutex: self }
    }

    /// Attempts to acquire this lock.
    ///
    /// If the lock could not be acquire at this time, then [`Err`] is returned. Otherwise, a RAII guard is returned.
    /// The lock will be unlocked when the guard is dropped.
    ///
    /// This function does not block.
    ///
    /// # Errors
    /// If the [`SpinLock`] could not be acquire because it is already locked, then this call will return an [`Err`].
    pub fn try_lock(&self) -> Result<SpinLockGuard<T>, ()> {
        if !self.lock.load(Ordering::Relaxed)
            && self
                .lock
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
        {
            return Ok(SpinLockGuard { mutex: self });
        }

        Err(())
    }

    /// Method that makes unlocking a mutex more explicit.
    pub fn unlock(guard: SpinLockGuard<T>) {
        drop(guard);
    }

    /// Returns a mutable reference to the underlying data.
    ///
    /// Since this call borrows the [`SpinLock`] mutably, no actual locking needs to take place
    /// - the mutable borrow statically guarantees no locks exist.
    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }
}

/// A RAII implementation of a "scoped lock" of a [`SpinLock`]. When this structure is dropped, the
/// lock will be unlcoked.
///
/// The data protected by the mutex can be access through this guard via its [`Deref`] and [`DerefMut`] implementations.
///
/// This structure is created by the [`SpinLock::lock()`] and [`SpinLock::try_lock()`] methods.
#[allow(clippy::module_name_repetitions)]
pub struct SpinLockGuard<'a, T: ?Sized> {
    /// The spinlock with which this [`SpinLockGuard`] is associated
    mutex: &'a SpinLock<T>,
}

impl<T: ?Sized> Deref for SpinLockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let value_ptr = self.mutex.value.get();

        // SAFETY:
        // We have exclusive access to the value pointed to by `value_ptr`.
        unsafe { &*value_ptr }
    }
}

impl<T: ?Sized> DerefMut for SpinLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let value_ptr = self.mutex.value.get();

        // SAFETY:
        // We have exclusive access to the value pointed to by `value_ptr`.
        unsafe { &mut *value_ptr }
    }
}

impl<T: ?Sized> Drop for SpinLockGuard<'_, T> {
    fn drop(&mut self) {
        self.mutex.lock.store(false, Ordering::Release);
    }
}
