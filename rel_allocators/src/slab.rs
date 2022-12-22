use ::core::{
    alloc::Layout,
    cell::Cell,
    ptr::{slice_from_raw_parts_mut, NonNull},
};
use ::heresy::alloc::AllocError;
use ::ptr_meta::PtrExt;
use ::rel_core::{Basis, DefaultBasis, Portable};

use crate::Control;

#[derive(Debug)]
pub struct SlabError;

#[derive(Portable)]
#[repr(C)]
pub struct Slab<B: Basis = DefaultBasis> {
    len: Cell<B::Usize>,
}

impl<B: Basis> Slab<B> {
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        B::to_native_usize(self.len.get()).unwrap()
    }
}

unsafe impl<B: Basis> Control for Slab<B>
where
    B::Usize: Portable,
{
    unsafe fn new(_: NonNull<[u8]>) -> Self {
        Self {
            len: Cell::new(B::from_native_usize(0).unwrap()),
        }
    }

    unsafe fn allocate(
        &self,
        memory: NonNull<[u8]>,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        let (ptr, cap) = PtrExt::to_raw_parts(memory.as_ptr());
        if ptr as usize & (layout.align() - 1) != 0 {
            Err(AllocError)
        } else {
            let len = B::to_native_usize(self.len.get()).unwrap();
            let start = (len + layout.align() - 1) & !(layout.align() - 1);
            let available = cap - start;
            if available < layout.size() {
                Err(AllocError)
            } else {
                self.len
                    .set(B::from_native_usize(start + layout.size()).unwrap());
                let address = unsafe { ptr.cast::<u8>().add(start) };
                let slice_ptr =
                    slice_from_raw_parts_mut(address, layout.size());
                Ok(unsafe { NonNull::new_unchecked(slice_ptr) })
            }
        }
    }

    unsafe fn deallocate(
        &self,
        _memory: NonNull<[u8]>,
        _ptr: NonNull<u8>,
        _layout: Layout,
    ) {
    }
}
