use ::core::{alloc::Layout, ptr::NonNull};
use ::heresy::alloc::{AllocError, Allocator};
use ::mischief::Slot;

use crate::{ContiguousAllocator, Control};

#[derive(Debug)]
pub struct ExternalError;

pub struct External<'a, C> {
    bytes: Slot<'a, [u8]>,
    control: C,
}

impl<'a, C: Control> External<'a, C> {
    const MIN_ALIGN: usize = 16;

    pub fn control(&self) -> &C {
        &self.control
    }

    fn memory(&self) -> NonNull<[u8]> {
        unsafe { NonNull::new_unchecked(self.bytes.as_ptr()) }
    }

    pub fn new(bytes: Slot<'a, [u8]>) -> Result<Self, ExternalError> {
        let ptr = unsafe { NonNull::new_unchecked(bytes.as_ptr()) };

        if ptr.as_ptr().cast::<u8>() as usize & (Self::MIN_ALIGN - 1) != 0 {
            Err(ExternalError)
        } else {
            let control = unsafe { C::new(ptr) };

            Ok(Self { bytes, control })
        }
    }
}

unsafe impl<'a, C: Control> Allocator for External<'a, C> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.control.allocate(self.memory(), layout) }
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { self.control.deallocate(self.memory(), ptr, layout) }
    }

    fn allocate_zeroed(
        &self,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.control.allocate_zeroed(self.memory(), layout) }
    }

    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe {
            self.control
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
            self.control
                .grow_zeroed(self.memory(), ptr, old_layout, new_layout)
        }
    }

    unsafe fn grow_in_place(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe {
            self.control.grow_in_place(
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
            self.control.grow_zeroed_in_place(
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
            self.control
                .shrink(self.memory(), ptr, old_layout, new_layout)
        }
    }

    unsafe fn shrink_in_place(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe {
            self.control.shrink_in_place(
                self.memory(),
                ptr,
                old_layout,
                new_layout,
            )
        }
    }
}

unsafe impl<C: Control> ContiguousAllocator for External<'_, C> {}
