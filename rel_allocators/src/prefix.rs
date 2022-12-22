//! An allocator that places its control structure at the beginning of its
//! memory segment.

use ::core::{
    alloc::Layout,
    marker::{PhantomData, PhantomPinned},
    ptr::{slice_from_raw_parts_mut, NonNull},
};
use ::heresy::alloc::{AllocError, Allocator};
use ::mischief::{In, Region, RegionalAllocator, Slot, Unique};
use ::munge::munge;
use ::ptr_meta::Pointee;
use ::rel_alloc::alloc::RelAllocator;
use ::rel_core::{
    Basis,
    DefaultBasis,
    Emplace,
    EmplaceExt,
    Move,
    Portable,
    RelRef,
};
use ::situ::{
    alloc::{RawAllocator, RawRegionalAllocator},
    DropRaw,
    Ref,
};

use crate::{
    unique_region::UniqueRegion,
    ContiguousAllocator,
    Control,
    RawContiguousAllocator,
};

#[derive(Debug)]
pub struct PrefixError;

#[derive(Portable)]
#[repr(C, align(16))]
struct PrefixHeader<C, B: Basis = DefaultBasis> {
    cap: B::Usize,
    control: C,
    _pinned: PhantomPinned,
}

impl<C, B: Basis> PrefixHeader<C, B> {
    const LAYOUT: Layout = Layout::new::<Self>();

    fn cap(&self) -> usize {
        B::to_native_usize(self.cap).unwrap()
    }

    fn memory(this: Ref<'_, Self>) -> NonNull<[u8]> {
        let ptr = unsafe {
            this.as_ptr()
                .cast::<u8>()
                .cast_mut()
                .add(Self::LAYOUT.size())
        };
        unsafe {
            NonNull::new_unchecked(slice_from_raw_parts_mut(ptr, this.cap()))
        }
    }

    fn split_prefix(
        slot: Slot<'_, [u8]>,
    ) -> Result<(Slot<'_, Self>, Slot<'_, [u8]>), PrefixError> {
        let len = slot.len();
        let ptr = slot.as_ptr() as *mut u8 as usize;
        if len < Self::LAYOUT.size() || ptr & (Self::LAYOUT.align() - 1) != 0 {
            Err(PrefixError)
        } else {
            let suffix_ptr =
                unsafe { slot.as_ptr().cast::<u8>().add(Self::LAYOUT.size()) };
            let suffix = unsafe {
                Slot::new_unchecked(slice_from_raw_parts_mut(
                    suffix_ptr,
                    len - Self::LAYOUT.size(),
                ))
            };
            let prefix = unsafe { slot.cast::<PrefixHeader<C, B>>() };
            Ok((prefix, suffix))
        }
    }
}

pub struct Prefix<'a, C, R: Region, B: Basis = DefaultBasis> {
    header: In<Ref<'a, PrefixHeader<C, B>>, R>,
}

impl<'a, C, R: Region, B: Basis> Clone for Prefix<'a, C, R, B> {
    fn clone(&self) -> Self {
        Self {
            header: self.header,
        }
    }
}

impl<'a, C, R: Region, B: Basis> Copy for Prefix<'a, C, R, B> {}

impl<'a, C: 'a, R: Region, B: 'a + Basis> Prefix<'a, C, R, B> {
    pub fn control(&self) -> &C {
        &self.header.control
    }

    fn memory(&self) -> NonNull<[u8]> {
        let header = In::into_inner(self.header);
        PrefixHeader::memory(header)
    }

    pub fn try_new_in(bytes: In<Slot<'a, [u8]>, R>) -> Result<Self, PrefixError>
    where
        C: Control,
    {
        let bytes = In::into_inner(bytes);

        let max_cap = bytes.len();
        let (mut prefix, suffix) = PrefixHeader::split_prefix(bytes)?;

        let suffix = unsafe { NonNull::new_unchecked(suffix.as_ptr()) };
        let control = unsafe { C::new(suffix) };

        munge!(
            let PrefixHeader {
                cap: mut out_cap,
                control: mut out_control,
                _pinned,
            } = prefix.as_mut()
        );

        out_cap.write(B::from_native_usize(max_cap).unwrap());
        out_control.write(control);

        let header_ref = unsafe { Ref::new_unchecked(prefix.as_ptr()) };
        let header = unsafe { In::new_unchecked(header_ref) };

        Ok(Prefix { header })
    }

    /// # Safety
    ///
    /// `bytes` must be a slice of bytes properly sized for the `Prefix`
    /// allocator located at its beginning.
    pub unsafe fn try_from_bytes(
        bytes: In<Slot<'a, [u8]>, R>,
    ) -> Result<Self, PrefixError> {
        let bytes = In::into_inner(bytes);
        let (prefix, _) = PrefixHeader::split_prefix(bytes)?;

        let header_ref = unsafe { Ref::new_unchecked(prefix.as_ptr()) };
        let header = unsafe { In::new_unchecked(header_ref) };

        Ok(Prefix { header })
    }
}

impl<'a, C: 'a + Control, U: Unique, B: 'a + Basis>
    Prefix<'a, C, UniqueRegion<'a, U>, B>
{
    pub fn try_new_in_region(
        bytes: Slot<'a, [u8]>,
        _: &'a mut U,
    ) -> Result<Self, PrefixError> {
        let bytes = unsafe { In::new_unchecked(bytes) };
        Self::try_new_in(bytes)
    }
}

pub struct PrefixRegion<U> {
    _phantom: PhantomData<U>,
}

unsafe impl<U: Unique> Region for PrefixRegion<U> {}

unsafe impl<C, R, B> Allocator for Prefix<'_, C, R, B>
where
    C: Control,
    R: Region,
    B: Basis,
{
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.header.control.allocate(self.memory(), layout) }
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { self.header.control.deallocate(self.memory(), ptr, layout) }
    }

    fn allocate_zeroed(
        &self,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.header.control.allocate_zeroed(self.memory(), layout) }
    }

    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe {
            self.header
                .control
                .grow(self.memory(), ptr, old_layout, new_layout)
        }
    }

    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe {
            self.header.control.grow_zeroed(
                self.memory(),
                ptr,
                old_layout,
                new_layout,
            )
        }
    }

    unsafe fn grow_in_place(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe {
            self.header.control.grow_in_place(
                self.memory(),
                ptr,
                old_layout,
                new_layout,
            )
        }
    }

    unsafe fn grow_zeroed_in_place(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe {
            self.header.control.grow_zeroed_in_place(
                self.memory(),
                ptr,
                old_layout,
                new_layout,
            )
        }
    }

    unsafe fn shrink(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe {
            self.header.control.shrink(
                self.memory(),
                ptr,
                old_layout,
                new_layout,
            )
        }
    }

    unsafe fn shrink_in_place(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe {
            self.header.control.shrink_in_place(
                self.memory(),
                ptr,
                old_layout,
                new_layout,
            )
        }
    }
}

unsafe impl<C: Control, R: Region, B: Basis> ContiguousAllocator
    for Prefix<'_, C, R, B>
{
}

unsafe impl<C, R, B> RegionalAllocator for Prefix<'_, C, R, B>
where
    C: Control,
    R: Region,
    B: Basis,
{
    type Region = R;
}

unsafe impl<'a, C, R, BH, BA> Emplace<RelPrefix<'a, C, R, BH, BA>, R>
    for Prefix<'a, C, R, BH>
where
    R: Region,
    BH: Basis,
    BA: Basis,
{
    fn emplaced_meta(
        &self,
    ) -> <RelPrefix<'a, C, R, BH, BA> as Pointee>::Metadata {
    }

    unsafe fn emplace_unsized_unchecked(
        self,
        out: In<Slot<'_, RelPrefix<'a, C, R, BH, BA>>, R>,
    ) {
        munge!(let RelPrefix { header } = out);
        self.header.emplace(header);
    }
}

#[derive(DropRaw, Move, Portable)]
#[repr(C)]
pub struct RelPrefix<
    'a,
    C,
    R: Region,
    BH: Basis = DefaultBasis,
    BA: Basis = DefaultBasis,
> {
    header: RelRef<'a, PrefixHeader<C, BH>, R, BA>,
}

unsafe impl<'a, C: Control, R: Region, BH: Basis, BA: Basis> RawAllocator
    for RelPrefix<'a, C, R, BH, BA>
{
    fn raw_allocate(
        this: Ref<'_, Self>,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        munge!(let Self { header, .. } = this);
        let header = RelRef::deref(header);
        unsafe {
            header
                .control
                .allocate(PrefixHeader::memory(header), layout)
        }
    }

    unsafe fn raw_deallocate(
        this: Ref<'_, Self>,
        ptr: NonNull<u8>,
        layout: Layout,
    ) {
        munge!(let Self { header, .. } = this);
        let header = RelRef::deref(header);
        unsafe {
            header
                .control
                .deallocate(PrefixHeader::memory(header), ptr, layout)
        }
    }

    fn raw_allocate_zeroed(
        this: Ref<'_, Self>,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        munge!(let Self { header, .. } = this);
        let header = RelRef::deref(header);
        unsafe {
            header
                .control
                .allocate_zeroed(PrefixHeader::memory(header), layout)
        }
    }

    unsafe fn raw_grow(
        this: Ref<'_, Self>,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        munge!(let Self { header, .. } = this);
        let header = RelRef::deref(header);
        unsafe {
            header.control.grow(
                PrefixHeader::memory(header),
                ptr,
                old_layout,
                new_layout,
            )
        }
    }

    unsafe fn raw_grow_zeroed(
        this: Ref<'_, Self>,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        munge!(let Self { header, .. } = this);
        let header = RelRef::deref(header);
        unsafe {
            header.control.grow_zeroed(
                PrefixHeader::memory(header),
                ptr,
                old_layout,
                new_layout,
            )
        }
    }

    unsafe fn raw_grow_in_place(
        this: Ref<'_, Self>,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        munge!(let Self { header, .. } = this);
        let header = RelRef::deref(header);
        unsafe {
            header.control.grow_in_place(
                PrefixHeader::memory(header),
                ptr,
                old_layout,
                new_layout,
            )
        }
    }

    unsafe fn raw_grow_zeroed_in_place(
        this: Ref<'_, Self>,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        munge!(let Self { header, .. } = this);
        let header = RelRef::deref(header);
        unsafe {
            header.control.grow_zeroed_in_place(
                PrefixHeader::memory(header),
                ptr,
                old_layout,
                new_layout,
            )
        }
    }

    unsafe fn raw_shrink(
        this: Ref<'_, Self>,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        munge!(let Self { header, .. } = this);
        let header = RelRef::deref(header);
        unsafe {
            header.control.shrink(
                PrefixHeader::memory(header),
                ptr,
                old_layout,
                new_layout,
            )
        }
    }

    unsafe fn raw_shrink_in_place(
        this: Ref<'_, Self>,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        munge!(let Self { header, .. } = this);
        let header = RelRef::deref(header);
        unsafe {
            header.control.shrink_in_place(
                PrefixHeader::memory(header),
                ptr,
                old_layout,
                new_layout,
            )
        }
    }
}

unsafe impl<C: Control, R: Region, BH: Basis, BA: Basis> RawContiguousAllocator
    for RelPrefix<'_, C, R, BH, BA>
{
}

unsafe impl<'a, C: Control, R: Region, BH: Basis, BA: Basis>
    RawRegionalAllocator for RelPrefix<'a, C, R, BH, BA>
{
    type Region = R;
}

unsafe impl<'a, C, R, BH, BA> RelAllocator<RelPrefix<'a, C, R, BH, BA>, R>
    for Prefix<'a, C, R, BH>
where
    C: Control,
    R: Region,
    BH: Basis,
    BA: Basis,
{
}
