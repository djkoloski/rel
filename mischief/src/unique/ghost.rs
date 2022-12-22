use ::core::marker::PhantomData;

use crate::unique::Unique;

/// A mutable "ghost" reference that is guaranteed to be zero-sized and obey
/// borrowing and ownership semantics
pub struct GhostMut<'a, T>(PhantomData<&'a mut T>);

impl<'a, T> GhostMut<'a, T> {
    /// Returns a new `GhostMut` from the given mutable reference.
    pub fn new(_: &'a mut T) -> Self {
        Self(PhantomData)
    }

    /// Returns a new `GhostMut` of the same value.
    pub fn as_mut(&mut self) -> GhostMut<'_, T> {
        GhostMut(PhantomData)
    }

    /// Converts this `GhostMut` into a `GhostRef` that borrows the same value
    /// for the same lifetime.
    pub fn into_ref(self) -> GhostRef<'a, T> {
        GhostRef(PhantomData)
    }

    /// Returns a new `GhostRef` that borrows the same value as this `GhostMut`.
    pub fn as_ref(&self) -> GhostRef<'_, T> {
        GhostRef(PhantomData)
    }
}

// SAFETY: Because `GhostMut` can only be constructed by leaking a `&mut T`, and
// `T` is guaranteed to be `Unique`, the `GhostMut` is also `Unique`.
unsafe impl<T: Unique> Unique for GhostMut<'_, T> {}

/// A "ghost" reference that is guaranteed to be zero-sized and obey borrowing
/// and ownership semantics
#[repr(transparent)]
pub struct GhostRef<'a, T: ?Sized>(PhantomData<&'a T>);

impl<T: ?Sized> Clone for GhostRef<'_, T> {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl<T: ?Sized> Copy for GhostRef<'_, T> {}

impl<'a, T: ?Sized> GhostRef<'a, T> {
    /// Returns a new `GhostRef` from the given shared reference.
    pub fn new(_: &'a T) -> Self {
        Self(PhantomData)
    }
}
