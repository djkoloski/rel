use ::core::{alloc::Layout, ptr::NonNull};
use ::heresy::alloc::AllocError;
use ::rel_core::Portable;

/// The control structure of an allocator.
///
/// `Control` structures can perform memory allocation operations on the memory
/// segment associated with their instance.
///
/// # Safety
///
/// Memory blocks returned from a control structure must point to valid memory
/// within its given region.
pub unsafe trait Control: Portable {
    /// Creates a new control structure for a memory segment.
    ///
    /// The returned control structure is specific to the memory segment it is
    /// created for.
    ///
    /// # Safety
    ///
    /// `memory` must be non-null, properly-aligned and point to a slice of `u8`
    /// that is valid for reads and writes.
    unsafe fn new(memory: NonNull<[u8]>) -> Self;

    /// Attempts to allocate a block of memory.
    ///
    /// See [`Allocator::allocate`] for more details.
    ///
    /// # Safety
    ///
    /// `memory` must be the memory segment specific to this control structure.
    unsafe fn allocate(
        &self,
        memory: NonNull<[u8]>,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError>;

    /// Deallocates the memory referenced by `ptr`.
    ///
    /// See [`Allocator::deallocate`] for more details.
    ///
    /// # Safety
    ///
    /// - `memory` must be the memory segment specific to this control
    ///   structure.
    /// - `ptr` must denote a block of memory _currently allocated_ via this
    ///    control structure.
    /// - `layout` must _fit_ that block of memory.
    unsafe fn deallocate(
        &self,
        memory: NonNull<[u8]>,
        ptr: NonNull<u8>,
        layout: Layout,
    );

    /// Behaves like `allocate`, but also ensures that the returned memory is
    /// zero-initialized.
    ///
    /// See [`Allocator::allocate_zeroed`] for more details.
    ///
    /// # Safety
    ///
    /// `memory` must be the memory segment specific to this control structure.
    unsafe fn allocate_zeroed(
        &self,
        memory: NonNull<[u8]>,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        // SAFETY: The caller has guaranteed that `memory` is the memory segment
        // specific to this control structure.
        let ptr = unsafe { self.allocate(memory, layout)? };
        let len = ::ptr_meta::metadata(ptr.as_ptr());
        // SAFETY: `alloc` returned a valid memory block of length `len`.
        unsafe {
            ptr.as_ptr().cast::<u8>().write_bytes(0, len);
        }
        Ok(ptr)
    }

    /// Attempts to extend the memory block.
    ///
    /// See [`Allocator::grow`] for more details.
    ///
    /// # Safety
    ///
    /// - `memory` must be the memory segment specific to this control
    ///   structure.
    /// - `ptr` must denote a block of memory _currently allocated_ via this
    ///   control structure.
    /// - `old_layout` must _fit_ that block of memory (the `new_layout`
    ///   argument need not fit it).
    /// - `new_layout.size()` must be greater than or equal to
    ///   `old_layout.size()`.
    unsafe fn grow(
        &self,
        memory: NonNull<[u8]>,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        debug_assert!(
            new_layout.size() >= old_layout.size(),
            "`new_layout.size()` must be greater than or equal to \
            `old_layout.size()`",
        );

        // SAFETY: `grow_in_place` has the same safety requirements as `grow`.
        let result =
            unsafe { self.grow_in_place(memory, ptr, old_layout, new_layout) };
        result.or_else(|_| {
            // SAFETY: The caller has guaranteed that `memory` is the memory
            // segment specific to this control structure.
            let new_ptr = unsafe { self.allocate(memory, new_layout)? };

            // SAFETY:
            // - The caller has guaranteed that `old_layout` fits the memory
            //   pointed to by `ptr`, and so must be valid for reads of
            //   `old_layout.size()`.
            // - The caller has guaranteed that `new_layout.size()` is
            //   greater than or equal to `old_layout.size()`, so `new_ptr`
            //   must be valid for writes of `old_layout.size()`.
            // - `u8` has an alignment of 1, so both pointers must be
            //   properly aligned.
            // - The memory pointed by `new_ptr` is freshly-allocated and
            //   must not overlap with the memory pointed to by `old_ptr`.
            unsafe {
                ::core::ptr::copy_nonoverlapping(
                    ptr.as_ptr(),
                    new_ptr.as_ptr().cast::<u8>(),
                    old_layout.size(),
                );
            }
            // SAFETY:
            // - The caller has guaranteed that `memory` is the memory segment
            //   specific to this control structure.
            // - The caller has guaranteed that `ptr` denotes a block
            //   of memory currently allocated via this control structure.
            // - The caller has guaranteed that `old_layout` fits that block of
            //   memory.
            unsafe {
                self.deallocate(memory, ptr, old_layout);
            }

            Ok(new_ptr)
        })
    }

    /// Behaves like `grow`, but also ensures that the new contents are set to
    /// zero before being returned.
    ///
    /// See [`Allocator::grow_zeroed`] for more details.
    ///
    /// # Safety
    ///
    /// - `memory` must be the memory segment specific to this control
    ///   structure.
    /// - `ptr` must denote a block of memory currently allocated via this
    ///   control structure.
    /// - `old_layout` must _fit_ that block of memory (the `new_layout`
    ///   argument need not fit it).
    /// - `new_layout.size()` must be greater than or equal to
    ///   `old_layout.align()`.
    unsafe fn grow_zeroed(
        &self,
        memory: NonNull<[u8]>,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        debug_assert!(
            new_layout.size() >= old_layout.size(),
            "`new_layout.size()` must be greater than or equal to `old_layout.size()`",
        );

        let result =
            // SAFETY: `grow_zeroed_in_place` has the same safety requirements
            // as `grow_zeroed`.
            unsafe { self.grow_zeroed_in_place(memory, ptr, old_layout, new_layout) };
        result.or_else(|_| {
            // SAFETY: The caller has guaranteed that `memory` is the memory
            // segment specific to this control structure.
            let new_ptr = unsafe { self.allocate(memory, new_layout)? };

            // SAFETY:
            // - The caller has guaranteed that `old_layout` fits the memory
            //   pointed to by `ptr`, and so must be valid for reads of
            //   `old_layout.size()`.
            // - The caller has guaranteed that `new_layout.size()` is greater
            //   than or equal to `old_layout.size()`, so `new_ptr` must be
            //   valid for writes of `old_layout.size()`.
            // - `u8` has an alignment of 1, so both pointers must be properly
            //   aligned.
            // - The memory pointed by `new_ptr` is freshly-allocated and must
            //   not overlap with the memory pointed to by `old_ptr`.
            unsafe {
                ::core::ptr::copy_nonoverlapping(
                    ptr.as_ptr(),
                    new_ptr.as_ptr().cast::<u8>(),
                    old_layout.size(),
                );
            }
            // SAFETY:
            // - The end of the old bytes is followed by `new_size - old_size`
            //   bytes which are valid for writes.
            // - A `u8` pointer is always properly aligned.
            unsafe {
                ::core::ptr::write_bytes(
                    new_ptr.as_ptr().cast::<u8>().add(old_layout.size()),
                    0,
                    new_layout.size() - old_layout.size(),
                );
            }
            // SAFETY:
            // - The caller has guaranteed that `memory` is the memory segment
            //   specific to this control structure.
            // - The caller has guaranteed that `ptr` denotes a block
            //   of memory currently allocated via this control structure.
            // - The caller has guaranteed that `old_layout` fits that block of
            //   memory.
            unsafe {
                self.deallocate(memory, ptr, old_layout);
            }

            Ok(new_ptr)
        })
    }

    /// Behaves like `grow` but returns `Err` if the memory block cannot be
    /// grown in-place.
    ///
    /// See [`Allocator::grow_in_place`] for more details.
    ///
    /// # Safety
    ///
    /// - `memory` must be the memory segment specific to this control
    ///   structure.
    /// - `ptr` must denote a block of memory _currently allocated_ via this
    ///   control structure.
    /// - `old_layout` must _fit_ that block of memory (the `new_layout`
    ///   argument need not fit it).
    /// - `new_layout.size()` must be greater than or equal to
    ///   `old_layout.size()`.
    unsafe fn grow_in_place(
        &self,
        memory: NonNull<[u8]>,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        let _ = (memory, ptr, old_layout, new_layout);

        Err(AllocError)
    }

    /// Behaves like `grow_zeroed` but returns `Err` if the memory block cannot
    /// be grown in-place.
    ///
    /// # Safety
    ///
    /// - `memory` must be the memory segment specific to this control
    ///   structure.
    /// - `ptr` must denote a block of memory _currently allocated_ via this
    ///   control structure.
    /// - `old_layout` must _fit_ that block of memory (the `new_layout`
    ///   argument need not fit it).
    /// - `new_layout.size()` must be greater than or equal to
    ///   `old_layout.size()`.
    unsafe fn grow_zeroed_in_place(
        &self,
        memory: NonNull<[u8]>,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        let new_ptr =
            // SAFETY: `grow_in_place` has the same safety requirements as
            // `grow_zeroed_in_place`.
            unsafe { self.grow_in_place(memory, ptr, old_layout, new_layout)? };

        // SAFETY:
        // - The end of the old bytes is followed by `new_size - old_size` bytes
        //   which are valid for writes.
        // - A `u8` pointer is always properly aligned.
        unsafe {
            ::core::ptr::write_bytes(
                new_ptr.as_ptr().cast::<u8>().add(old_layout.size()),
                0,
                new_layout.size() - old_layout.size(),
            );
        }

        Ok(new_ptr)
    }

    /// Attempts to shrink the memory block.
    ///
    /// See [`Allocator::shrink`] for more details.
    ///
    /// # Safety
    ///
    /// - `memory` must be the memory segment specific to this control
    ///   structure.
    /// - `ptr` must denote a block of memory _currently allocated_ by this
    ///   control structure.
    /// - `old_layout` must _fit_ that block of memory (The `new_layout`
    ///   argument need not fit it).
    /// - `new_layout.size()` must be less than or equal to `old_layout.size()`.
    unsafe fn shrink(
        &self,
        memory: NonNull<[u8]>,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        debug_assert!(
            new_layout.size() <= old_layout.size(),
            "`new_layout.size()` must be less than or equal to `old_layout.size()`",
        );

        let result =
            // SAFETY: `shrink_in_place` has the same safety requirements as
            // `shrink`.
            unsafe { self.shrink_in_place(memory, ptr, old_layout, new_layout) };
        result.or_else(|_| {
            // SAFETY: The caller has guaranteed that `memory` is the memory
            // segment specific to this control structure.
            let new_ptr = unsafe { self.allocate(memory, new_layout)? };

            // SAFETY:
            // - The caller has guaranteed that `old_layout` fits the memory
            //   pointed to by `ptr`, and `new_layout.size()` is less than or
            //   equal to `old_layout.size()`, so `ptr` must be valid for reads
            //   of `new_layout.size()`.
            // - `new_ptr` points to a memory block at least `new_layout.size()`
            //   in length, so `new_ptr` must be valid for writes of
            //   `new_layout.size()`.
            // - `u8` has an alignment of 1, so both pointers must be properly
            //   aligned.
            // - The memory pointed by `new_ptr` is freshly-allocated and must
            //   not overlap with the memory pointed to by `old_ptr`.
            unsafe {
                ::core::ptr::copy_nonoverlapping(
                    ptr.as_ptr(),
                    new_ptr.as_ptr().cast::<u8>(),
                    new_layout.size(),
                );
            }
            // SAFETY:
            // - The caller has guaranteed that `memory` is the memory segment
            //   specific to this control structure.
            // - The caller has guaranteed that `ptr` denotes a block
            //   of memory currently allocated via this control structure.
            // - The caller has guaranteed that `old_layout` fits that block of
            //   memory.
            unsafe {
                self.deallocate(memory, ptr, old_layout);
            }

            Ok(new_ptr)
        })
    }

    /// Behaves like `shrink` but returns `Err` if the memory block cannot be
    /// shrunk in-place.
    ///
    /// See [`Allocator::shrink_in_place`] for more details.
    ///
    /// # Safety
    ///
    /// - `memory` must be the memory segment specific to this control
    ///   structure.
    /// - `ptr` must denote a block of memory _currently allocated_ by this
    ///   control structure.
    /// - `old_layout` must _fit_ that block of memory (The `new_layout`
    ///   argument need not fit it).
    /// - `new_layout.size()` must be less than or equal to `old_layout.size()`.
    unsafe fn shrink_in_place(
        &self,
        memory: NonNull<[u8]>,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        let _ = (memory, ptr, old_layout, new_layout);

        Err(AllocError)
    }
}
