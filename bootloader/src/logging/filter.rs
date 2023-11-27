//! Provides support for atomic modification of [`LevelFilter`]s.

use core::sync::atomic::{AtomicU8, Ordering};

use log::LevelFilter;

/// A [`LevelFilter`] which can be safely shared between threads.
#[expect(
    clippy::module_name_repetitions,
    reason = "repeating the name makes it much clearer what its function is"
)]
pub struct AtomicLevelFilter(AtomicU8);

impl AtomicLevelFilter {
    /// Creates a new atomic level filter.
    pub const fn new(level_filter: LevelFilter) -> AtomicLevelFilter {
        AtomicLevelFilter(AtomicU8::new(to_u8(level_filter)))
    }

    /// Stores a [`LevelFilter`] into the atomic level filter.
    ///
    /// `store` takes an [`Ordering`] argument which describes the memory ordering
    ///  of this operation. Possible values are [`SeqCst`][sc],
    /// [`Release`][rs] and [`Relaxed`][rx].
    ///
    /// # Panics
    /// Panics if order is [`Acquire`][ac] or [`AcqRel`][ar].
    ///
    /// [sc]: Ordering::SeqCst
    /// [rs]: Ordering::Release
    /// [rx]: Ordering::Relaxed
    /// [ac]: Ordering::Acquire
    /// [ar]: Ordering::AcqRel
    pub fn store(&self, level_filter: LevelFilter, order: Ordering) {
        self.0.store(to_u8(level_filter), order);
    }

    /// Loads a [`LevelFilter`] from the atomic level filter.
    ///
    /// `load` takes an [`Ordering`] argument which describes the memory ordering
    ///  of this operation. Possible values are [`SeqCst`][sc],
    /// [`Acquire`][ac] and [`Relaxed`][rx].
    ///
    /// # Panics
    /// Panics if order is [`Release`][rs] or [`AcqRel`][ar].
    ///
    /// [sc]: Ordering::SeqCst
    /// [ac]: Ordering::Acquire
    /// [rx]: Ordering::Relaxed
    /// [rs]: Ordering::Release
    /// [ar]: Ordering::AcqRel
    pub fn load(&self, order: Ordering) -> LevelFilter {
        /// The integer value of [`LevelFilter::Off`].
        const OFF: u8 = to_u8(LevelFilter::Off);
        /// The integer value of [`LevelFilter::Error`].
        const ERROR: u8 = to_u8(LevelFilter::Error);
        /// The integer value of [`LevelFilter::Warn`].
        const WARN: u8 = to_u8(LevelFilter::Warn);
        /// The integer value of [`LevelFilter::Info`].
        const INFO: u8 = to_u8(LevelFilter::Info);
        /// The integer value of [`LevelFilter::Debug`].
        const DEBUG: u8 = to_u8(LevelFilter::Debug);
        /// The integer value of [`LevelFilter::Trace`].
        const TRACE: u8 = to_u8(LevelFilter::Trace);

        let value = self.0.load(order);

        match value {
            OFF => LevelFilter::Off,
            ERROR => LevelFilter::Error,
            WARN => LevelFilter::Warn,
            INFO => LevelFilter::Info,
            DEBUG => LevelFilter::Debug,
            TRACE => LevelFilter::Trace,
            _ => unreachable!(),
        }
    }
}

/// Converts a [`LevelFilter`] to its integer representation.
const fn to_u8(level_filter: LevelFilter) -> u8 {
    match level_filter {
        LevelFilter::Off => LevelFilter::Off as u8,
        LevelFilter::Error => LevelFilter::Error as u8,
        LevelFilter::Warn => LevelFilter::Warn as u8,
        LevelFilter::Info => LevelFilter::Info as u8,
        LevelFilter::Debug => LevelFilter::Debug as u8,
        LevelFilter::Trace => LevelFilter::Trace as u8,
    }
}
