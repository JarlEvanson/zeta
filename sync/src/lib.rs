//! Implementation of common no-std synchronization primitives.

#![no_std]

use core::{
    cell::UnsafeCell,
    fmt,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicU8, Ordering},
};

/// A mutual-exclusion primitive useful for protecting shared data.
///
/// This mutex implements a fast-path for uncontended locks, a slower path
/// for periods of micro-contention, and a fallback to exponentional backoff
/// during high contention.
pub struct Mutex<T> {
    /// Storage for the two mutex-control flags.
    ///
    /// LOCK: Indicates if the mutex is currently locked.
    /// CONTENDED: Indicates if the mutex is currently contended.
    lock: AtomicU8,
    /// The value protected by the mutex.
    value: UnsafeCell<T>,
}

// SAFETY:
// Mutexes are safe to send across thread boundaries as long as the underlying type
// is safe to send across thread boundaries. Otherwise, using `mem::swap()` on a
// mutex guard would allow for dropping or using T.
unsafe impl<T: Send> Send for Mutex<T> {}
// SAFETY:
// Mutexes are safe to share across thread boundaries so long as the underlying type is
// `Send`, otherwise one could send T across boundaries using `mem::swap()` on a mutex guard.
unsafe impl<T: Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    /// A bit flag representing if the mutex is locked.
    const IS_LOCKED: u8 = 0b01;
    /// A bit flag representing if the mutex is contended.
    const IS_CONTENDED: u8 = 0b10;
    /// The maximum number of increments which are done before a retry is attempted
    /// when the fallback path occurs.
    const MAX_SLEEP: usize = 500_000_000;

    /// Constructs a new unlocked mutex.
    pub const fn new(value: T) -> Mutex<T> {
        Mutex {
            lock: AtomicU8::new(0),
            value: UnsafeCell::new(value),
        }
    }

    /// Acquires a mutex, spinning until acquisition succeeds.
    pub fn lock(&self) -> MutexGuard<T> {
        let current_state = self.lock.load(Ordering::Relaxed);

        if current_state & Self::IS_LOCKED == 0
            && self
                .lock
                .compare_exchange(
                    current_state,
                    current_state | Self::IS_LOCKED,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                )
                .is_ok()
        {
            return MutexGuard { lock: self };
        }

        for _ in 0..40 {
            if self.lock.load(Ordering::Relaxed) & Self::IS_CONTENDED == Self::IS_CONTENDED {
                break;
            }

            if self
                .lock
                .compare_exchange_weak(0, Self::IS_LOCKED, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                return MutexGuard { lock: self };
            }

            core::hint::spin_loop();
        }

        let mut current_state = self.lock.load(Ordering::Relaxed);

        loop {
            match self.lock.compare_exchange(
                current_state,
                current_state | Self::IS_CONTENDED,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(state) => current_state = state,
            }
        }

        let mut sleep = 3000;

        loop {
            let mut counter = 0;
            for _ in 0..sleep {
                counter = core::hint::black_box(counter) + 1;
            }

            let current_state = self.lock.load(Ordering::Relaxed);

            if current_state & Self::IS_LOCKED == 0
                && self
                    .lock
                    .compare_exchange(
                        current_state,
                        Self::IS_LOCKED,
                        Ordering::Acquire,
                        Ordering::Relaxed,
                    )
                    .is_ok()
            {
                return MutexGuard { lock: self };
            }

            if sleep < Self::MAX_SLEEP {
                sleep.checked_mul(2).unwrap_or(Self::MAX_SLEEP);
                sleep = sleep.min(Self::MAX_SLEEP);
            }
        }
    }
}

/// A RAII implementation of a scoped lock of a mutex. When this structure is dropped,
/// the lock will be unlocked.  
///
/// The data protected by the mutex can be access by its [`Deref`] and [`DerefMut`]
/// implementations.
pub struct MutexGuard<'lock, T> {
    /// The mutex which this guard belongs to.
    lock: &'lock Mutex<T>,
}

impl<T: fmt::Debug> fmt::Debug for MutexGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <T as fmt::Debug>::fmt(&**self, f)
    }
}

impl<'lock, T> Deref for MutexGuard<'lock, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY:
        // The existance of this guard proves that no other guard for this mutex
        // exists, and the immutable access to the guard proves that the value
        // has no mutable reference to it.
        unsafe { &*self.lock.value.get() }
    }
}

impl<'lock, T> DerefMut for MutexGuard<'lock, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY:
        // The existance of this guard proves that no other guard for this mutex
        // exists, and the mutable access to the guard proves that the value is unaliased.
        unsafe { &mut *self.lock.value.get() }
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.lock.store(0, Ordering::Release);
    }
}
