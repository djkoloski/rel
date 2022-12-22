use ::core::{alloc::Layout, ops::Deref, ptr::NonNull};
use ::heresy::alloc::{AllocError, Allocator};
use ::mischief::{In, Region, Slot, Static, StaticRef};
use ::munge::munge;
use ::ptr_meta::Pointee;
use ::rel_alloc::alloc::RelAllocator;
use ::rel_core::{Emplace, EmplaceExt, Move, Portable};
use ::situ::{alloc::RawAllocator, ops::DerefRaw, DropRaw, Ref};

use crate::{ContiguousAllocator, RawContiguousAllocator};

/// # Safety
///
/// When `Self` is emplaced as `E` in `R`, it must `DerefRaw` to the same value
/// as it did before. This also implies that `deref()` is stable.
pub unsafe trait RelDeref<E, R>: Deref + Emplace<E, R>
where
    E: DerefRaw + DropRaw,
    R: Region,
{
}

unsafe impl<'a, S, R> RelDeref<StaticRef<'a, S>, R> for StaticRef<'a, S>
where
    S: Static,
    R: Region,
{
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct DerefAdapter<P> {
    ptr: P,
}

impl<P> DerefAdapter<P> {
    pub fn new(ptr: P) -> Self {
        Self { ptr }
    }
}

impl<P: Deref> Deref for DerefAdapter<P> {
    type Target = P::Target;

    fn deref(&self) -> &Self::Target {
        self.ptr.deref()
    }
}

unsafe impl<P, EP, R> Emplace<RelDerefAdapter<EP>, R> for DerefAdapter<P>
where
    P: Emplace<EP, R>,
    EP: DropRaw,
    R: Region,
{
    fn emplaced_meta(&self) -> <RelDerefAdapter<EP> as Pointee>::Metadata {}

    unsafe fn emplace_unsized_unchecked(
        self,
        out: In<Slot<'_, RelDerefAdapter<EP>>, R>,
    ) {
        munge!(let RelDerefAdapter { ptr } = out);
        self.ptr.emplace(ptr);
    }
}

unsafe impl<P: Deref> Allocator for DerefAdapter<P>
where
    P::Target: Allocator,
{
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.deref().allocate(layout)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { self.deref().deallocate(ptr, layout) }
    }

    fn allocate_zeroed(
        &self,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        self.deref().allocate_zeroed(layout)
    }

    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.deref().grow(ptr, old_layout, new_layout) }
    }

    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.deref().grow_zeroed(ptr, old_layout, new_layout) }
    }

    unsafe fn grow_in_place(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.deref().grow_in_place(ptr, old_layout, new_layout) }
    }

    unsafe fn grow_zeroed_in_place(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe {
            self.deref()
                .grow_zeroed_in_place(ptr, old_layout, new_layout)
        }
    }

    unsafe fn shrink(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.deref().shrink(ptr, old_layout, new_layout) }
    }

    unsafe fn shrink_in_place(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.deref().shrink_in_place(ptr, old_layout, new_layout) }
    }
}

unsafe impl<P> ContiguousAllocator for DerefAdapter<P>
where
    P: Deref,
    P::Target: ContiguousAllocator,
{
}

#[derive(DropRaw, Portable, Move)]
#[repr(transparent)]
pub struct RelDerefAdapter<P> {
    ptr: P,
}

unsafe impl<P: DerefRaw> RawAllocator for RelDerefAdapter<P>
where
    P::Target: RawAllocator,
{
    fn raw_allocate(
        this: Ref<'_, Self>,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        munge!(let Self { ptr: p } = this);
        let alloc = DerefRaw::deref_raw(p);
        RawAllocator::raw_allocate(alloc, layout)
    }

    unsafe fn raw_deallocate(
        this: Ref<'_, Self>,
        ptr: NonNull<u8>,
        layout: Layout,
    ) {
        munge!(let Self { ptr: p } = this);
        let alloc = DerefRaw::deref_raw(p);
        unsafe { RawAllocator::raw_deallocate(alloc, ptr, layout) }
    }

    fn raw_allocate_zeroed(
        this: Ref<'_, Self>,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        munge!(let Self { ptr: p } = this);
        let alloc = DerefRaw::deref_raw(p);
        RawAllocator::raw_allocate_zeroed(alloc, layout)
    }

    unsafe fn raw_grow(
        this: Ref<'_, Self>,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        munge!(let Self { ptr: p } = this);
        let alloc = DerefRaw::deref_raw(p);
        unsafe { RawAllocator::raw_grow(alloc, ptr, old_layout, new_layout) }
    }

    unsafe fn raw_grow_zeroed(
        this: Ref<'_, Self>,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        munge!(let Self { ptr: p } = this);
        let alloc = DerefRaw::deref_raw(p);
        unsafe {
            RawAllocator::raw_grow_zeroed(alloc, ptr, old_layout, new_layout)
        }
    }

    unsafe fn raw_grow_in_place(
        this: Ref<'_, Self>,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        munge!(let Self { ptr: p } = this);
        let alloc = DerefRaw::deref_raw(p);
        unsafe {
            RawAllocator::raw_grow_in_place(alloc, ptr, old_layout, new_layout)
        }
    }

    unsafe fn raw_grow_zeroed_in_place(
        this: Ref<'_, Self>,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        munge!(let Self { ptr: p } = this);
        let alloc = DerefRaw::deref_raw(p);
        unsafe {
            RawAllocator::raw_grow_zeroed_in_place(
                alloc, ptr, old_layout, new_layout,
            )
        }
    }

    unsafe fn raw_shrink(
        this: Ref<'_, Self>,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        munge!(let Self { ptr: p } = this);
        let alloc = DerefRaw::deref_raw(p);
        unsafe { RawAllocator::raw_shrink(alloc, ptr, old_layout, new_layout) }
    }

    unsafe fn raw_shrink_in_place(
        this: Ref<'_, Self>,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        munge!(let Self { ptr: p } = this);
        let alloc = DerefRaw::deref_raw(p);
        unsafe {
            RawAllocator::raw_shrink_in_place(
                alloc, ptr, old_layout, new_layout,
            )
        }
    }
}

unsafe impl<P> RawContiguousAllocator for RelDerefAdapter<P>
where
    P: DerefRaw,
    P::Target: RawContiguousAllocator,
{
}

unsafe impl<P, EP, R> RelAllocator<RelDerefAdapter<EP>, R> for DerefAdapter<P>
where
    P: RelDeref<EP, R>,
    P::Target: Allocator,
    EP: DerefRaw + DropRaw,
    EP::Target: RawAllocator,
    R: Region,
{
}
