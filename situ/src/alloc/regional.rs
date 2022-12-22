use ::mischief::{Region, RegionalAllocator};

use crate::alloc::RawAllocator;

/// A `RawAllocator` that allocates inside a single contiguous memory region.
///
/// # Safety
///
/// The pointers returned from a `RawRegionalAllocator`'s `RawAllocator`
/// implementation must always be contained in its associated `Region`.
pub unsafe trait RawRegionalAllocator: RawAllocator {
    /// The region type for this allocator.
    type Region: Region;
}

// SAFETY: The `RawAllocator` impl for `T` proxies the `Allocator` impl for `T`
// and the `Region` for `RegionalAllocator` is the same as the `Region` for
// `RawRegionalAllocator`.
unsafe impl<T: RegionalAllocator> RawRegionalAllocator for T {
    type Region = T::Region;
}
