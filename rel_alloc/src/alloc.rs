//! Memory allocation APIs.

use ::heresy::alloc::Allocator;
use ::mischief::Region;
use ::ptr_meta::Pointee;
use ::rel_core::Emplace;
use ::situ::{alloc::RawAllocator, DropRaw};

/// An `Allocator` that is suitable for allocating relative types.
///
/// # Safety
///
/// Implementing `RelAllocator` guarantees that emplacing some `E` in `R`
/// creates a value that functions analogously to the original allocator.
/// Specifically, it must return the same results when calling the analogous
/// allocator methods from `RawAllocator` and share the same state between the
/// two (e.g. allocating with one and freeing with the other must be safe and
/// function properly).
pub unsafe trait RelAllocator<E, R>: Allocator + Emplace<E, R>
where
    E: DropRaw + Pointee + RawAllocator + ?Sized,
    R: Region,
{
}
