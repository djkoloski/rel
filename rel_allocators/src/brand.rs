use ::core::{alloc::Layout, marker::PhantomData, ops::Deref, ptr::NonNull};
use ::heresy::alloc::{AllocError, Allocator};
use ::mischief::{In, Region, RegionalAllocator, Slot, Unique};
use ::munge::munge;
use ::ptr_meta::Pointee;
use ::rel_alloc::alloc::RelAllocator;
use ::rel_core::{Emplace, EmplaceExt, Move, Portable};
use ::situ::{
    alloc::{RawAllocator, RawRegionalAllocator},
    DropRaw,
    Ref,
};

use crate::{
    adapters::DerefAdapter,
    unique_region::UniqueRegion,
    ContiguousAllocator,
    RawContiguousAllocator,
};

// Two avenues for making allocators:
// - `In<Slot<[u8]>, R>`: Use the existing `R` region and compose it with the
//   contiguous allocator.
// - `Slot<[u8]>`: Create a fresh `Region` type and compose it with the
//   contiguous allocator to create a regional allocator. This effectively
//   forges a new `In<Slot<[u8]>, R>` and calls the existing version with it.

pub struct Brand<A, R> {
    alloc: A,
    region: PhantomData<R>,
}

impl<A, R> Brand<A, R> {
    /// # Safety
    ///
    /// `alloc` must always return memory located in `R`.
    pub unsafe fn new_unchecked(alloc: A) -> Self {
        Self {
            alloc,
            region: PhantomData,
        }
    }

    pub fn inner(&self) -> &A {
        &self.alloc
    }
}

impl<P, R> Brand<DerefAdapter<P>, R> {
    /// # Safety
    ///
    /// `ptr` must always deref to an allocator that returns memory located in
    /// `R`.
    pub unsafe fn new_deref_unchecked(ptr: P) -> Self
    where
        P: Deref,
        P::Target: Allocator,
    {
        unsafe { Self::new_unchecked(DerefAdapter::new(ptr)) }
    }
}

impl<'a, A, U: Unique> Brand<A, UniqueRegion<'a, U>> {
    pub fn new(alloc: A, _: &'a mut U) -> Self {
        Self {
            alloc,
            region: PhantomData,
        }
    }
}

impl<'a, P, U: Unique> Brand<DerefAdapter<P>, UniqueRegion<'a, U>> {
    pub fn new_deref(ptr: P, unique: &'a mut U) -> Self
    where
        P: Deref,
        P::Target: Allocator,
    {
        Self::new(DerefAdapter::new(ptr), unique)
    }
}

impl<A: Clone, R> Clone for Brand<A, R> {
    fn clone(&self) -> Self {
        Self {
            alloc: self.alloc.clone(),
            region: self.region,
        }
    }
}

impl<A: Copy, R> Copy for Brand<A, R> {}

unsafe impl<A, R> Allocator for Brand<A, R>
where
    A: Allocator,
{
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.alloc.allocate(layout)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { self.alloc.deallocate(ptr, layout) }
    }

    fn allocate_zeroed(
        &self,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        self.alloc.allocate_zeroed(layout)
    }

    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.alloc.grow(ptr, old_layout, new_layout) }
    }

    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.alloc.grow_zeroed(ptr, old_layout, new_layout) }
    }

    unsafe fn grow_in_place(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.alloc.grow_in_place(ptr, old_layout, new_layout) }
    }

    unsafe fn grow_zeroed_in_place(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.alloc.grow_zeroed_in_place(ptr, old_layout, new_layout) }
    }

    unsafe fn shrink(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.alloc.shrink(ptr, old_layout, new_layout) }
    }

    unsafe fn shrink_in_place(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.alloc.shrink_in_place(ptr, old_layout, new_layout) }
    }
}

unsafe impl<A: ContiguousAllocator, R: Region> RegionalAllocator
    for Brand<A, R>
{
    type Region = R;
}

unsafe impl<A, R, EA, ER> Emplace<RelBrand<EA, R>, ER> for Brand<A, R>
where
    A: Emplace<EA, ER>,
    EA: DropRaw,
    ER: Region,
{
    fn emplaced_meta(&self) -> <RelBrand<A, R> as Pointee>::Metadata {}

    unsafe fn emplace_unsized_unchecked(
        self,
        out: In<Slot<'_, RelBrand<EA, R>>, ER>,
    ) {
        munge!(let RelBrand { alloc, region } = out);
        self.alloc.emplace(alloc);
        self.region.emplace(region);
    }
}

#[derive(DropRaw, Portable, Move)]
#[repr(C)]
pub struct RelBrand<A, R> {
    alloc: A,
    region: PhantomData<R>,
}

unsafe impl<A, R> RawAllocator for RelBrand<A, R>
where
    A: RawAllocator,
{
    fn raw_allocate(
        this: Ref<'_, Self>,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        munge!(let Self { alloc, .. } = this);
        A::raw_allocate(alloc, layout)
    }

    unsafe fn raw_deallocate(
        this: Ref<'_, Self>,
        ptr: NonNull<u8>,
        layout: Layout,
    ) {
        munge!(let Self { alloc, .. } = this);
        unsafe { A::raw_deallocate(alloc, ptr, layout) }
    }

    fn raw_allocate_zeroed(
        this: Ref<'_, Self>,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        munge!(let Self { alloc, .. } = this);
        A::raw_allocate_zeroed(alloc, layout)
    }

    unsafe fn raw_grow(
        this: Ref<'_, Self>,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        munge!(let Self { alloc, .. } = this);
        unsafe { A::raw_grow(alloc, ptr, old_layout, new_layout) }
    }

    unsafe fn raw_grow_zeroed(
        this: Ref<'_, Self>,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        munge!(let Self { alloc, .. } = this);
        unsafe { A::raw_grow_zeroed(alloc, ptr, old_layout, new_layout) }
    }

    unsafe fn raw_grow_in_place(
        this: Ref<'_, Self>,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        munge!(let Self { alloc, .. } = this);
        unsafe { A::raw_grow_in_place(alloc, ptr, old_layout, new_layout) }
    }

    unsafe fn raw_grow_zeroed_in_place(
        this: Ref<'_, Self>,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        munge!(let Self { alloc, .. } = this);
        unsafe {
            A::raw_grow_zeroed_in_place(alloc, ptr, old_layout, new_layout)
        }
    }

    unsafe fn raw_shrink(
        this: Ref<'_, Self>,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        munge!(let Self { alloc, .. } = this);
        unsafe { A::raw_shrink(alloc, ptr, old_layout, new_layout) }
    }

    unsafe fn raw_shrink_in_place(
        this: Ref<'_, Self>,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        munge!(let Self { alloc, .. } = this);
        unsafe { A::raw_shrink_in_place(alloc, ptr, old_layout, new_layout) }
    }
}

unsafe impl<A, R> RawRegionalAllocator for RelBrand<A, R>
where
    A: RawContiguousAllocator,
    R: Region,
{
    type Region = R;
}

unsafe impl<A, R, EA, ER> RelAllocator<RelBrand<EA, R>, ER> for Brand<A, R>
where
    A: RelAllocator<EA, ER>,
    R: Region,
    EA: DropRaw + RawAllocator,
    ER: Region,
{
}
