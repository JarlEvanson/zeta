//! A UEFI boot services-based vector.

use core::{
    alloc::Layout,
    fmt::Debug,
    marker::PhantomData,
    mem::{self, MaybeUninit},
    ptr::NonNull,
};

use uefi::{
    boot::{acquire_boot_handle, BootHandle},
    table::boot::MemoryType,
};

/// A UEFI boot services-based vector.
pub struct Vec<T> {
    /// The state of the allocation to hold the elements stored in itself.
    allocated: Allocated<T>,
    /// The total amount of initialized elements that the buffer pointed to
    /// by `allocated` can hold.
    capacity: usize,
    /// The number of initialized elements pointed to by `allocated`.
    len: usize,
    /// Phantom marker.
    phantom: PhantomData<T>,
}

impl<T> Vec<T> {
    /// Constructs a new, empty `Vec<T>`.
    #[must_use]
    pub const fn new() -> Vec<T> {
        Self {
            allocated: Allocated::Unallocated,
            capacity: 0,
            len: 0,
            phantom: PhantomData,
        }
    }

    /// Constructs a new, empty `Vec<T>` with at least the specified capacity.
    ///
    /// The vector will be able to hold at least `capacity` elements of `T`.
    ///
    /// # Errors
    /// - Returns [`CapacityOverflow`][co] if the computed capacity would exceed `isize::MAX` bytes.
    /// - Returns [`AllocError`][ae] if an error occured during allocation.
    ///
    /// [co]: TryWithCapacityError::CapacityOverflow
    /// [ae]: TryWithCapacityError::AllocError
    pub fn with_capacity(capacity: usize) -> Result<Vec<T>, TryWithCapacityError> {
        if capacity == 0 {
            return Ok(Vec::new());
        }

        let Ok(layout) = Layout::array::<T>(capacity) else {
            return Err(TryWithCapacityError::CapacityOverflow);
        };

        let actual_size = required_buf_size(8, mem::align_of::<T>(), layout.size())
            .ok_or(TryWithCapacityError::CapacityOverflow)?;

        let boot_handle = acquire_boot_handle();

        let ptr = match boot_handle
            .allocate_pool(MemoryType::LOADER_DATA, actual_size)
            .map(NonNull::new)
        {
            Ok(Some(ptr)) => ptr.cast::<T>(),
            Err(_) | Ok(None) => {
                return Err(TryWithCapacityError::AllocError { size: actual_size })
            }
        };

        let vec = Vec {
            allocated: Allocated::Allocated {
                ptr,
                handle: boot_handle,
            },
            capacity,
            len: 0,
            phantom: PhantomData,
        };

        Ok(vec)
    }

    /// Returns a buffer to the buffer aligned to the start of the first element.
    fn get_aligned_ptr(&self) -> Option<NonNull<T>> {
        match self.allocated {
            Allocated::Unallocated => None,
            Allocated::Allocated { ptr, handle: _ } => {
                NonNull::new(align_up_pointer(ptr.as_ptr())?)
            }
        }
    }

    /// Returns the remaining spare capacity of the vector as a slice of `MaybeUninit<T>`.
    pub fn spare_capacity_mut(&mut self) -> &mut [MaybeUninit<T>] {
        if let Some(ptr) = self.get_aligned_ptr() {
            // SAFETY:
            // `ptr` points to a buffer that can hold at least `len` elements, but up to `capacity` elements.
            let ptr = unsafe { ptr.as_ptr().add(self.len) }.cast::<MaybeUninit<T>>();

            let spare_capacity = self.capacity - self.len;

            // SAFETY:
            // - `MaybeUninit` is always valid for reads and writes when properly aligned.
            // - The difference between `capacity` and `len` is always positive and represents
            //      the amount of uninitialized elements in the buffer.
            unsafe { core::slice::from_raw_parts_mut(ptr, spare_capacity) }
        } else {
            &mut []
        }
    }

    /// Returns a slice of the initialized elements that the vector holds.
    #[must_use]
    pub fn as_slice(&self) -> &[T] {
        if let Some(ptr) = self.get_aligned_ptr() {
            // SAFETY:
            // `ptr` is properly aligned and by the invariants of `Vec`,
            // points to `len` initialized elements.
            unsafe { core::slice::from_raw_parts(ptr.as_ptr(), self.len) }
        } else {
            &[]
        }
    }

    /// Returns a mutable slice of the initialized elements that the vector holds.
    #[must_use]
    pub fn as_slice_mut(&mut self) -> &mut [T] {
        if let Some(ptr) = self.get_aligned_ptr() {
            // SAFETY:
            // `ptr` is properly aligned and by the invariants of `Vec`,
            // points to `len` initialized elements.
            unsafe { core::slice::from_raw_parts_mut(ptr.as_ptr(), self.len) }
        } else {
            &mut []
        }
    }

    /// Attempts to reserve space for `additional_capacity` extra elements beyond the current `len`.
    ///
    /// # Errors
    /// - Returns [`CapacityOverflow`][co] if the computed capacity would exceed `isize::MAX` bytes.
    /// - Returns [`AllocError`][ae] if an error occured during allocation.
    ///
    /// [co]: TryReserveError::CapacityOverflow
    /// [ae]: TryReserveError::AllocError
    #[expect(
        clippy::missing_panics_doc,
        reason = "allocated addresses must be capable of being aligned"
    )]
    pub fn try_reserve(&mut self, additional_capacity: usize) -> Result<(), TryReserveError> {
        if additional_capacity == 0 {
            return Ok(());
        }

        let new_capacity = if let Some(new_capacity) = self.len.checked_add(additional_capacity) {
            if new_capacity <= self.capacity {
                return Ok(());
            }
            new_capacity
        } else {
            return Err(TryReserveError::CapacityOverflow);
        };

        let Ok(layout) = Layout::array::<T>(new_capacity) else {
            return Err(TryReserveError::CapacityOverflow);
        };

        let actual_size = required_buf_size(8, mem::align_of::<T>(), layout.size())
            .ok_or(TryReserveError::CapacityOverflow)?;

        match &mut self.allocated {
            Allocated::Allocated { ptr, handle } => {
                let new_ptr = match handle
                    .allocate_pool(MemoryType::LOADER_DATA, actual_size)
                    .map(NonNull::new)
                {
                    Ok(Some(new_ptr)) => new_ptr.cast::<T>(),
                    Err(_) | Ok(None) => {
                        return Err(TryReserveError::AllocError { size: actual_size })
                    }
                };

                let aligned_new_ptr = align_up_pointer(new_ptr.as_ptr()).unwrap();
                let aligned_old_ptr = align_up_pointer(ptr.as_ptr()).unwrap();

                // SAFETY:
                // `aligned_old_ptr` points to at least `self.len` initialized elements.
                // `aligned_new_ptr` points to space for at least `new_capacity` elements.
                // `aligned_old_ptr` and `aligned_new_ptr` have both been aligned.
                // `aligned_old_ptr` and `aligned_new_ptr` belong to different allocations, so
                // they are distinct.
                unsafe {
                    core::ptr::copy_nonoverlapping(aligned_old_ptr, aligned_new_ptr, self.len);
                }

                // SAFETY:
                // No more references exist to `ptr`, since we have mutable access to the vector.
                unsafe { handle.free_pool(ptr.as_ptr().cast::<u8>()) }
                    .expect("failed to free pool memory");

                *ptr = new_ptr;
                self.capacity = new_capacity;
            }
            Allocated::Unallocated => {
                let boot_handle = acquire_boot_handle();

                let new_ptr = match boot_handle
                    .allocate_pool(MemoryType::LOADER_DATA, actual_size)
                    .map(NonNull::new)
                {
                    Ok(Some(ptr)) => ptr.cast::<T>(),
                    Err(_) | Ok(None) => {
                        return Err(TryReserveError::AllocError { size: actual_size })
                    }
                };

                self.allocated = Allocated::Allocated {
                    ptr: new_ptr,
                    handle: boot_handle,
                };
                self.capacity = new_capacity;
            }
        };

        Ok(())
    }

    pub fn push_within_capacity(&mut self, value: T) -> Result<(), T> {
        let spare_capacity = self.spare_capacity_mut();

        if spare_capacity.is_empty() {
            return Err(value);
        }

        unsafe {
            spare_capacity[0].write(value);
        }

        unsafe { self.set_len(self.len + 1) }

        Ok(())
    }

    /// Forces the length of the vector to `new_len`.
    ///
    /// This is a low-level operation that maintains none of the normal invariants
    /// of the type.
    ///
    /// # Safety
    /// `new_len` must be less than or equal to [`self.capacity()`][c].
    /// The elements at `old_len..new_len` must be initialized.
    ///
    /// [c]: Self::capacity
    pub unsafe fn set_len(&mut self, new_len: usize) {
        self.len = new_len;
    }

    /// Returns the total capacity of the vector.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the current number of initialized elements in the vector.
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the vector is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns a slice pointing to the initialized elements that `self` holds.
    #[must_use]
    pub fn leak(self) -> (&'static mut [T], usize) {
        let Self {
            allocated: _,
            capacity,
            len,
            phantom: _,
        } = self;

        let slice = if let Some(ptr) = self.get_aligned_ptr() {
            // SAFETY:
            // `ptr` is properly aligned and by the invariants of `Vec`,
            // points to `len` initialized elements.
            unsafe { core::slice::from_raw_parts_mut(ptr.as_ptr(), len) }
        } else {
            &mut []
        };

        (slice, capacity)
    }

    /// Returns a slice pointing to the buffer that `self` had.
    #[must_use]
    pub fn leak_maybe_uninit(self) -> (&'static mut [MaybeUninit<T>], usize) {
        let Self {
            allocated: _,
            capacity,
            len,
            phantom: _,
        } = self;

        let slice = if let Some(ptr) = self.get_aligned_ptr() {
            // SAFETY:
            // `ptr` is properly aligned and by the invariants of `Vec`,
            // points to `len` initialized elements.
            unsafe { core::slice::from_raw_parts_mut(ptr.cast::<MaybeUninit<T>>().as_ptr(), len) }
        } else {
            &mut []
        };

        (slice, capacity)
    }

    /// Returns the raw parts of the vector `(ptr, capacity, len)`.
    ///
    /// `ptr` can be unaligned when `mem::align_of::<T> > 8`, and so to access the elements,
    /// `ptr` must first be aligned up, and then `ptr` can be treated as a ptr to the start of a slice.
    #[must_use]
    pub fn into_raw_parts(self) -> (Option<NonNull<T>>, usize, usize) {
        let Self {
            ref allocated,
            capacity,
            len,
            phantom: _,
        } = self;

        if let Allocated::Allocated { ptr, handle: _ } = allocated {
            (Some(*ptr), capacity, len)
        } else {
            (None, capacity, len)
        }
    }
}

impl<T: Debug> Debug for Vec<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut darr = f.debug_list();

        darr.entries(self.as_slice());

        darr.finish()
    }
}

impl<T> Default for Vec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for Vec<T> {
    fn drop(&mut self) {
        let aligned_ptr = self.get_aligned_ptr();
        match &mut self.allocated {
            Allocated::Allocated { ptr, handle } => {
                let ptr = ptr.as_ptr();

                if core::mem::needs_drop::<T>() {
                    let mut item_ptr = aligned_ptr.unwrap().as_ptr();

                    for _ in 0..self.len {
                        // SAFETY:
                        // - All safe methods of changing len ensure initialization of
                        // elements `0..new_len`, thus `item_ptr` is properly initialized.
                        // - `item_ptr` has been properly aligned.
                        // - `item_ptr` is nonnull, since it was allocated from a pool and
                        //      does not wrap around.
                        unsafe { core::ptr::drop_in_place(item_ptr) }

                        // SAFETY:
                        // All safe methods of changing `len` ensure that `capacity` is at
                        // least as great as `len`, which means that `item_ptr` can be
                        // incremented by one at least `len` times and not break invariants.
                        item_ptr = unsafe { item_ptr.add(1) }
                    }
                }

                // SAFETY:
                // We have a mutable reference to the vector, which means
                // we have exclusive access to the contents of the vector.
                // Therefore, no references to the allocation exist.
                //
                // While access to the allocation could happen after freed,
                // such a thing would have to use a pointer or unsafely convert into a reference.
                unsafe { handle.free_pool(ptr.cast::<u8>()) }.expect("deallocation failed");
            }
            Allocated::Unallocated => {}
        }
    }
}

/// The internal allocation state of a vector.
enum Allocated<T> {
    /// The buffer has been allocated.
    Allocated {
        /// The pointer to the buffer.
        ptr: NonNull<T>,
        /// Handle to boot services to ensure deallocation is possible.
        handle: BootHandle,
    },
    /// The buffer has not been allocated.
    Unallocated,
}

/// Various errors that can occur when calling [`Vec::with_capacity`].
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum TryWithCapacityError {
    /// Error due to the computed capacity exceeding the collection's maximum
    /// (usually `isize::MAX` bytes)
    CapacityOverflow,

    /// An error occurred while attempting to allocate a buffer.
    AllocError {
        /// Number of bytes in the allocation request that failed.
        size: usize,
    },
}
/// Various errors that can occur when calling [`Vec::try_reserve`].
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum TryReserveError {
    /// Error due to the computed capacity exceeding the collection's maximum
    /// (usually `isize::MAX` bytes)
    CapacityOverflow,

    /// An error occurred while attempting to allocate a buffer.
    AllocError {
        /// Number of bytes in the allocation request that failed.
        size: usize,
    },
}

/// Calculates the actual buffer size required to allocate a buffer capable of storing
/// `byte_count` bytes aligned to `guaranteed_alignment` when the allocator only guarantees
/// `required_alignment`.
///
/// Checks that `required_buf_size` is less than or equal to `isize::MAX`.
const fn required_buf_size(
    guaranteed_alignment: usize,
    required_alignment: usize,
    byte_count: usize,
) -> Option<usize> {
    let max_waste = required_alignment.saturating_sub(guaranteed_alignment);

    let Some(buf_size) = byte_count.checked_add(max_waste) else {
        return None;
    };

    if buf_size > isize::MAX as usize {
        return None;
    }

    Some(buf_size)
}

/// Aligns the pointer to the next properly aligned slot.
///
/// Does not change the pointer if `ptr` is already aligned.
fn align_up_pointer<T>(ptr: *mut T) -> Option<*mut T> {
    ptr.addr()
        .checked_next_multiple_of(mem::align_of::<T>())
        .map(|new_addr| ptr.with_addr(new_addr))
}
