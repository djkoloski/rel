use ::core::{marker::PhantomData, mem::ManuallyDrop, ptr};
use ::mischief::{GhostRef, In, Region, Slot, Static, StaticRef};
use ::ptr_meta::Pointee;
use ::situ::DropRaw;

use crate::{Emplace, EmplaceExt};

macro_rules! impl_builtin {
    ($($ty:ty),*) => {
        $(
            // SAFETY:
            // - `emplaced_meta` returns `()`, the only valid metadata for
            //  `Sized` types.
            // - `emplace_unsized_unchecked` initializes its `out` parameter by
            //   writing to it.
            unsafe impl<R: Region> Emplace<$ty, R> for $ty {
                fn emplaced_meta(&self) -> <Self as Pointee>::Metadata {}

                unsafe fn emplace_unsized_unchecked(
                    self,
                    out: In<Slot<'_, $ty>, R>,
                ) {
                    In::into_inner(out).write(self);
                }
            }
        )*
    }
}

impl_builtin!(i8, u8, bool, ());

// SAFETY:
// - `emplaced_meta` returns `()`, the only valid metadata for `Sized` types.
// - `PhantomData<T>` is a zero-sized type and so is already initialized.
unsafe impl<T, R> Emplace<PhantomData<T>, R> for PhantomData<T>
where
    T: ?Sized,
    R: Region,
{
    fn emplaced_meta(&self) -> <PhantomData<T> as Pointee>::Metadata {}

    unsafe fn emplace_unsized_unchecked(
        self,
        _: In<Slot<'_, PhantomData<T>>, R>,
    ) {
    }
}

// SAFETY:
// - `emplaced_meta` returns `()`, the only valid metadata for `Sized` types.
// - `emplace_unsized_unchecked` emplaces to every element of the `out` slot,
//   which initializes it.
unsafe impl<E, T, R: Region, const N: usize> Emplace<[T; N], R> for [E; N]
where
    E: Emplace<T, R>,
    T: DropRaw,
{
    fn emplaced_meta(&self) -> <Self as Pointee>::Metadata {}

    unsafe fn emplace_unsized_unchecked(self, out: In<Slot<'_, [T; N]>, R>) {
        let emplacers = ManuallyDrop::new(self);
        let mut out = In::into_inner(out);
        for i in 0..N {
            // SAFETY: `i` is in bounds because it must be less than the length
            // of the array, `N`.
            let out_i = unsafe { out.as_mut().get_unchecked(i) };
            // SAFETY: `out_i` is located in `R` because `out` is located in `R`
            // and `out_i` is an element of `out`.
            let out_i = unsafe { In::new_unchecked(out_i) };
            // SAFETY: The pointer being read is from a reference, so it must be
            // valid for reads, properly aligned, and point to an initialized
            // value.
            let emplacer_i = unsafe { ptr::read(&emplacers[i]) };
            emplacer_i.emplace(out_i);
        }
    }
}

// SAFETY:
// - `emplaced_meta` returns `()`, the only valid metadata for `Sized` types.
// - `GhostRef`s are always properly-initialized because they are zero-sized
//   types.
unsafe impl<T: ?Sized, R: Region> Emplace<Self, R> for GhostRef<'_, T> {
    fn emplaced_meta(&self) -> <Self as Pointee>::Metadata {}

    unsafe fn emplace_unsized_unchecked(self, _: In<Slot<'_, Self>, R>) {}
}

// SAFETY:
// - `emplaced_meta` returns `()`, the only valid metadata for `Sized` types.
// - `StaticRef`s are always properly-initialized because they are zero-sized
//   types.
unsafe impl<S: Static, R: Region> Emplace<Self, R> for StaticRef<'_, S> {
    fn emplaced_meta(&self) -> <Self as Pointee>::Metadata {}

    unsafe fn emplace_unsized_unchecked(self, _: In<Slot<'_, Self>, R>) {}
}
