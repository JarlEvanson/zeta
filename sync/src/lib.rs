#![no_std]

use core::{
    cell::UnsafeCell,
    fmt,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicU8, Ordering},
};

pub struct Mutex<T> {
    lock: AtomicU8,
    value: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for Mutex<T> {}
unsafe impl<T: Send> Send for Mutex<T> {}

impl<T> Mutex<T> {
    const IS_LOCKED: u8 = 0b01;
    const IS_CONTENDED: u8 = 0b10;
    const MAX_SLEEP: usize = 500_000_000;

    pub const fn new(value: T) -> Mutex<T> {
        Mutex {
            lock: AtomicU8::new(0),
            value: UnsafeCell::new(value),
        }
    }

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

pub struct MutexGuard<'lock, T> {
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
