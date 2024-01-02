//! An arena meant for temporary allocations.

use core::{
    cell::Cell,
    mem::{self, MaybeUninit},
    ptr::NonNull,
};

/// An arena allocator.
pub struct Arena {
    /// The start of the arena.
    start: Cell<NonNull<u8>>,
    /// Remaining number of bytes in the arena.
    ///
    /// Always less than  `isize::MAX`.
    len: Cell<usize>,
}

impl Arena {
    /// Creates a new [`Arena`].
    ///
    /// # Safety
    /// - The region of memory that `start` points to and extends for `len` bytes must not be used
    ///     until this [`Arena`] is dropped, except through this [`Arena`] and its created frames
    /// - The region of memory must be a single allocated object.
    /// - `len` must be less than or equal to [`isize::MAX`]
    pub unsafe fn new(start: NonNull<u8>, len: usize) -> Arena {
        Arena {
            start: Cell::new(start),
            len: Cell::new(len),
        }
    }

    /// Creates the base [`Frame`] for this [`Arena`].
    pub fn base_frame(&mut self) -> Frame {
        Frame {
            arena: self,
            frame_base: self.start.get(),
        }
    }

    /// Allocates enough bytes to store a single `T`.
    fn allocate_single<T>(&self) -> NonNull<MaybeUninit<T>> {
        self.alloc_internal(mem::size_of::<T>(), mem::align_of::<T>(), 1)
            .cast::<MaybeUninit<T>>()
    }

    /// Allocates enough bytes to storage `count` `T`s.
    fn allocate_slice<T>(&self, count: usize) -> NonNull<[MaybeUninit<T>]> {
        NonNull::slice_from_raw_parts(
            self.alloc_internal(mem::size_of::<T>(), mem::align_of::<T>(), count)
                .cast::<MaybeUninit<T>>(),
            count,
        )
    }

    /// The general purpose allocation interface, upon which all other allocation interfaces of [`Arena`] are built.
    #[expect(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
    fn alloc_internal(&self, size: usize, align: usize, count: usize) -> NonNull<u8> {
        let padding = (-(self.start.get().as_ptr().addr() as isize)) as usize & (align - 1);

        let available = self.len.get() as isize - padding as isize;

        assert!(
            !(available < 0 || count > available as usize / size),
            "arena out of memory"
        );

        // SAFETY:
        // The above bounds check means that this is safe.
        let pointer = unsafe { self.start.get().add(padding) };

        // SAFETY:
        // The above bounds check means that this is safe.
        let new_start = unsafe { pointer.add(count * size) };

        self.start.set(new_start);
        self.len.set(self.len.get() - padding - count * size);

        pointer
    }
}

/// A frame on the arena allocator, containing where the arena can be reset back to.
pub struct Frame<'arena> {
    /// The underlying [`Arena`] to which this [`Frame`] is tied.
    arena: &'arena Arena,
    /// The location of the pointer at the time of creation of this [`Frame`].
    ///
    /// When dropped, this is where the [`Arena`]'s pointer is reset to.
    frame_base: NonNull<u8>,
}

impl<'arena> Frame<'arena> {
    /// Creates a new [`Frame`] to allocate from.
    pub fn next_frame(&mut self) -> Frame {
        Frame {
            arena: self.arena,
            frame_base: self.arena.start.get(),
        }
    }

    /// Allocates enough bytes to store a single `T`.
    pub fn allocate_single<T>(&self) -> &'arena mut MaybeUninit<T> {
        let mut ptr = self.arena.allocate_single();

        // SAFETY:
        // - `ptr` is properly aligned.
        // - The region of memory is within the bounds of a single allocated object.
        // - `MaybeUninit` is always initialized.
        // - The aliasing rules are obeyed.
        unsafe { ptr.as_mut() }
    }

    /// Allocates enough bytes to storage `count` `T`s.
    pub fn allocate_slice<T>(&self, count: usize) -> &'arena mut [MaybeUninit<T>] {
        let mut ptr = self.arena.allocate_slice(count);

        // SAFETY:
        // - `ptr` is properly aligned.
        // - The region of memory is within the bounds of a single allocated object.
        // - `MaybeUninit` is always initialized.
        // - The aliasing rules are obeyed.
        unsafe { ptr.as_mut() }
    }
}

impl Drop for Frame<'_> {
    fn drop(&mut self) {
        // SAFETY:
        // - Both pointers are tied to the same underlying `Arena`, and thus are derived from the same allocated object
        // - Any integer distance is a multiple of the size of `u8`.
        let ptr_diff = unsafe { self.arena.start.get().offset_from(self.frame_base) };
        let ptr_diff = TryInto::<usize>::try_into(ptr_diff).unwrap();

        self.arena.start.set(self.frame_base);
        self.arena.len.set(self.arena.len.get() + ptr_diff);
    }
}

#[cfg(test)]
mod test {
    use core::mem::MaybeUninit;

    use super::Arena;

    #[test]
    fn miri_test() {
        let mut storage = [0u8; 4096];

        let mut arena = unsafe {
            Arena::new(
                core::ptr::NonNull::from(&mut storage).cast::<u8>(),
                core::mem::size_of_val(&storage),
            )
        };

        let mut base_frame = arena.base_frame();

        let a = base_frame.allocate_slice::<u8>(1024);
        a.fill(MaybeUninit::new(0));
        let b = base_frame.allocate_slice::<u8>(1024);
        b.fill(MaybeUninit::new(0));

        let next_frame = base_frame.next_frame();

        let c = next_frame.allocate_slice::<u8>(1024);
        c.fill(MaybeUninit::new(0));
        let d = next_frame.allocate_slice::<u8>(1024);
        d.fill(MaybeUninit::new(0));
    }

    #[test]
    #[should_panic]
    fn overflow() {
        let mut storage = [0u8; 4096];

        let mut arena = unsafe {
            Arena::new(
                core::ptr::NonNull::from(&mut storage).cast::<u8>(),
                core::mem::size_of_val(&storage),
            )
        };

        arena.base_frame().allocate_single::<[u8; 4097]>();
    }

    #[test]
    fn frame_work() {
        let mut storage = [0u8; 4096];

        let mut arena = unsafe {
            Arena::new(
                core::ptr::NonNull::from(&mut storage).cast::<u8>(),
                core::mem::size_of_val(&storage),
            )
        };

        let base_frame = arena.base_frame();

        let a = base_frame.allocate_slice::<u8>(1024);
        a.fill(MaybeUninit::new(0));

        drop(base_frame);

        let base_frame = arena.base_frame();

        let a = base_frame.allocate_slice::<u8>(1024);
        a.fill(MaybeUninit::new(0));

        drop(base_frame);
    }
}
