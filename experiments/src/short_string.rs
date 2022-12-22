//! A UTF-8 encoded, growable string with an optimization for short strings.

use ::core::{
    alloc::Layout,
    fmt,
    mem::MaybeUninit,
    ptr::{
        copy_nonoverlapping,
        slice_from_raw_parts,
        slice_from_raw_parts_mut,
    },
};
use ::mischief::{In, RegionalAllocator, Slot};
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
    RelPtr,
};
use ::situ::{
    alloc::RawRegionalAllocator,
    fmt::{DebugRaw, DisplayRaw},
    ops::{DerefMutRaw, DerefRaw},
    str::{from_raw_utf8_unchecked, from_raw_utf8_unchecked_mut},
    DropRaw,
    Mut,
    Ref,
    Val,
};

#[derive(DropRaw, Move, Portable)]
#[repr(C)]
struct Repr<A: RawRegionalAllocator, B: Basis> {
    ptr: RelPtr<u8, A::Region, B>,
    len: B::Usize,
}

/// A relative counterpart to `String`.
#[derive(Portable)]
#[repr(C)]
pub struct RelShortString<A: RawRegionalAllocator, B: Basis = DefaultBasis> {
    repr: MaybeUninit<Repr<A, B>>,
    cap: B::Usize,
    alloc: A,
}

impl<A: RawRegionalAllocator, B: Basis> DropRaw for RelShortString<A, B> {
    unsafe fn drop_raw(_this: Mut<'_, Self>) {
        todo!();
    }
}

impl<A: RawRegionalAllocator, B: Basis> RelShortString<A, B> {
    const INLINE_CAPACITY: usize = ::core::mem::size_of::<Repr<A, B>>();

    /// Returns a reference to the underlying allocator.
    #[inline]
    pub fn allocator(this: Ref<'_, Self>) -> Ref<'_, A> {
        munge!(let RelShortString { alloc, .. } = this);
        alloc
    }

    /// Returns a byte slice of this `RelString`'s contents.
    pub fn as_bytes(this: Ref<'_, Self>) -> Ref<'_, [u8]> {
        munge!(let RelShortString { repr, .. } = this);

        let bytes_ptr = if this.is_inline() {
            // Inlined
            repr.as_ptr().cast::<u8>()
        } else {
            // Not inlined
            // SAFETY: If the string isn't inline, then its repr is initialized.
            let repr = unsafe { Ref::assume_init(repr) };
            munge!(let Repr { ptr, .. } = repr);
            // SAFETY: If the string isn't inline, then its pointer is always
            // non-null.
            unsafe { RelPtr::as_ptr_unchecked(ptr) }
        };

        let slice_ptr = slice_from_raw_parts(bytes_ptr, this.len());
        // SAFETY:
        // - `slice_ptr` is always non-null and valid for reads. Because it
        //   points to a slice of `u8` (which have alignment 1), it is always
        //   properly aligned.
        // - `this` does not alias any other mutable references because it is a
        //   `Ref`, so the bytes it points to cannot either.
        // - The value pointed to by `slice_ptr` is either the interned bytes
        //   within `repr` or a separate allocation pointed to by `repr`. Both
        //   must be initialized.
        unsafe { Ref::new_unchecked(slice_ptr) }
    }

    /// Returns a string slice of the `RelString`'s contents.
    #[inline]
    pub fn as_str(this: Ref<'_, Self>) -> Ref<'_, str> {
        // SAFETY: The bytes of a `RelString` are always valid UTF-8.
        unsafe { from_raw_utf8_unchecked(Self::as_bytes(this)) }
    }

    /// Returns a mutable byte slice of this `RelString`'s contents.
    pub fn as_mut_bytes(this: Mut<'_, Self>) -> Mut<'_, [u8]> {
        let is_inline = this.is_inline();
        let len = this.len();

        munge!(let RelShortString { repr, .. } = this);

        let bytes_ptr = if is_inline {
            // Inlined
            repr.as_ptr().cast::<u8>()
        } else {
            // Not inlined
            // SAFETY: If the string isn't inline, then its repr is initialized.
            let repr = unsafe { Mut::assume_init(repr) };
            munge!(let Repr { ptr, .. } = repr);
            // SAFETY: If the string isn't inline, then its pointer is always
            // non-null.
            unsafe { RelPtr::as_mut_ptr_unchecked(ptr) }
        };

        let slice_ptr = slice_from_raw_parts_mut(bytes_ptr, len);
        // SAFETY:
        // - `slice_ptr` is always non-null and valid for reads and writes.
        //   Because it points to a slice of `u8` (which have alignment 1), it
        //   is always properly aligned.
        // - `this` does not alias any other accessible references because it is
        //   a `Ref`, so the bytes it points to cannot either.
        // - The value pointed to by `slice_ptr` is either the interned bytes
        //   within `repr` or a separate allocation pointed to by `repr`. Both
        //   must be initialized.
        unsafe { Mut::new_unchecked(slice_ptr) }
    }

    /// Returns a mutable string slice of the `RelString`'s contents.
    #[inline]
    pub fn as_mut_str(this: Mut<'_, Self>) -> Mut<'_, str> {
        let bytes = Self::as_mut_bytes(this);
        // SAFETY: The bytes of a `RelString` are always valid UTF-8.
        unsafe { from_raw_utf8_unchecked_mut(bytes) }
    }

    #[inline]
    fn internal_cap(&self) -> usize {
        B::to_native_usize(self.cap).unwrap()
    }

    #[inline]
    fn is_inline(&self) -> bool {
        self.internal_cap() <= Self::INLINE_CAPACITY
    }

    /// Returns this `RelString`'s capacity, in bytes.
    #[inline]
    pub fn capacity(&self) -> usize {
        usize::max(self.internal_cap(), Self::INLINE_CAPACITY)
    }

    /// Truncates this `RelString`, removing all contents.
    ///
    /// While this means the `String` will have a length of zero, it does not
    /// affect its capacity.
    #[inline]
    pub fn clear(this: Mut<'_, Self>) {
        let is_inline = this.is_inline();

        munge!(let RelShortString { repr, mut cap, .. } = this);

        if is_inline {
            // Inlined
            *cap = B::from_native_usize(Self::INLINE_CAPACITY).unwrap();
        } else {
            // Not inlined
            // SAFETY: If the string isn't inline, then its repr is initialized.
            let repr = unsafe { Mut::assume_init(repr) };
            munge!(let Repr { mut len, .. } = repr);
            *len = B::from_native_usize(0).unwrap();
        }
    }

    /// Returns the length of this `RelString`, in bytes, not `char`s or
    /// graphemes. In other words, it might not be what a human considers the
    /// length of the string.
    #[inline]
    pub fn len(&self) -> usize {
        if self.is_inline() {
            // Inlined
            self.internal_cap()
        } else {
            // Not inlined
            // SAFETY: If the string isn't inline, then its repr is initialized.
            let repr = unsafe { self.repr.assume_init_ref() };
            B::to_native_usize(repr.len).unwrap()
        }
    }

    /// Returns whether this `RelString` is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<A: RawRegionalAllocator, B: Basis> DerefRaw for RelShortString<A, B> {
    type Target = str;

    fn deref_raw(this: Ref<'_, Self>) -> Ref<'_, Self::Target> {
        Self::as_str(this)
    }
}

impl<A: RawRegionalAllocator, B: Basis> DerefMutRaw for RelShortString<A, B> {
    fn deref_mut_raw(this: Mut<'_, Self>) -> Mut<'_, Self::Target> {
        Self::as_mut_str(this)
    }
}

// SAFETY: TODO
unsafe impl<A: RawRegionalAllocator, B: Basis> Move<A::Region>
    for RelShortString<A, B>
{
    unsafe fn move_unsized_unchecked(
        _this: In<Val<'_, Self>, A::Region>,
        _out: In<Slot<'_, Self>, A::Region>,
    ) {
        todo!();
    }
}

/// An emplacer for a `RelString` that copies its bytes from a `str`.
pub struct Clone<'a, R>(pub R, pub &'a str);

// SAFETY:
// - `RelString` is `Sized` and always has metadata `()`, so `emplaced_meta`
//   always returns valid metadata for it.
// - `emplace_unsized_unchecked` initializes its `out` parameter by emplacing to
//   each field.
unsafe impl<A, B, R> Emplace<RelShortString<A, B>, R::Region> for Clone<'_, R>
where
    A: DropRaw + RawRegionalAllocator<Region = R::Region>,
    B: Basis,
    R: RegionalAllocator + RelAllocator<A, R::Region>,
{
    fn emplaced_meta(&self) -> <RelShortString<A, B> as Pointee>::Metadata {}

    unsafe fn emplace_unsized_unchecked(
        self,
        out: In<Slot<'_, RelShortString<A, B>>, A::Region>,
    ) {
        munge!(let RelShortString { repr, cap, alloc } = out);

        let len = self.1.len();

        if len <= RelShortString::<A, B>::INLINE_CAPACITY {
            // Inline
            In::into_inner(cap).write(B::from_native_usize(len).unwrap());
            let ptr = In::into_inner(repr).as_ptr().cast::<u8>();
            // SAFETY:
            // - `src.1.as_ptr()` is valid for reads of `len` bytes because it
            //   is a pointer to a `&str` of length `len`.
            // - `ptr` is valid for writes of `len` bytes because it was
            //   allocated with capacity `len`.
            // - Both `str` and `ptr` are allocated with the proper alignment
            //   for `u8`.
            // - The two regions of memory cannot overlap because `ptr` is part
            //   of the `out` slot which cannot be aliased.
            unsafe {
                copy_nonoverlapping(self.1.as_ptr(), ptr, len);
            }
        } else {
            // Not inline
            // SAFETY: `Slot::uninit` returns a pointer to the same slot, so it
            // must be located in the same region.
            let repr = unsafe { In::map_unchecked(repr, Slot::uninit) };
            munge!(let Repr { ptr: out_ptr, len: out_len } = repr);

            let ptr = self
                .0
                .allocate(Layout::array::<u8>(len).unwrap())
                .unwrap()
                .cast()
                .as_ptr();

            // SAFETY:
            // - `src.1.as_ptr()` is valid for reads of `len` bytes because it
            //   is a pointer to a `&str` of length `len`.
            // - `ptr` is valid for writes of `len` bytes because it was
            //   allocated with capacity `len`.
            // - Both `str` and `ptr` are allocated with the proper alignment
            //   for `u8`.
            // - The two regions of memory cannot overlap because `ptr` is newly
            //   allocated and points to unaliased memory.
            unsafe {
                copy_nonoverlapping(self.1.as_ptr(), ptr, len);
            }

            // SAFETY: The pointer returned from `allocate` is guaranteed to be
            // in the region of `R`.
            let ptr = unsafe { In::new_unchecked(ptr) };

            ptr.emplace(out_ptr);
            In::into_inner(out_len).write(B::from_native_usize(len).unwrap());
        }

        self.0.emplace(alloc);
    }
}

impl<A: RawRegionalAllocator, B: Basis> DebugRaw for RelShortString<A, B> {
    fn fmt_raw(
        this: Ref<'_, Self>,
        f: &mut fmt::Formatter<'_>,
    ) -> Result<(), fmt::Error> {
        fmt::Debug::fmt(&*Self::as_str(this), f)
    }
}

impl<A: RawRegionalAllocator, B: Basis> DisplayRaw for RelShortString<A, B> {
    fn fmt_raw(
        this: Ref<'_, Self>,
        f: &mut fmt::Formatter<'_>,
    ) -> Result<(), fmt::Error> {
        fmt::Display::fmt(&*Self::as_str(this), f)
    }
}
