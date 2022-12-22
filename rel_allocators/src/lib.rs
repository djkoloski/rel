#![deny(unsafe_op_in_unsafe_fn)]

pub mod adapters;
pub mod brand;
mod control;
pub mod external;
pub mod legacy;
pub mod prefix;
pub mod slab;
pub mod unique_region;

use ::heresy::alloc::Allocator;
use ::situ::alloc::RawAllocator;

pub use self::control::*;

/// # Safety
///
/// Types that implement `ContiguousAllocator` must always allocate memory
/// within a single contiguous region. Unlike `RegionalAllocator`, that region
/// is anonymous.
pub unsafe trait ContiguousAllocator: Allocator {}

/// # Safety
///
/// Types that implement `RawContiguousAllocator` must always allocate memory
/// within a single contiguous region. Unlike `RawRegionalAllocator`, that
/// region is anonymous.
pub unsafe trait RawContiguousAllocator: RawAllocator {}

unsafe impl<T: ContiguousAllocator> RawContiguousAllocator for T {}
