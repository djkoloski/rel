mod data;

use ::mischief::{
    lease_static,
    runtime_token,
    In,
    Region,
    RegionalAllocator,
    Slot,
    StaticToken,
    StaticVal,
};
use ::munge::munge;
use ::rand::Rng;
use ::rel_alloc::{alloc::RelAllocator, EmplaceIn, RelVec};
use ::rel_allocators::{
    brand::Brand,
    external::External,
    prefix::{Prefix, RelPrefix},
    slab::Slab,
};
use ::rel_core::{Emplace, EmplaceExt, Move, Portable, F32};
use ::rel_util::Align16;
use ::situ::{alloc::RawRegionalAllocator, DropRaw};

use crate::{benchmarks::*, from_data::FromData, gen::generate_vec};

#[derive(DropRaw, Move, Portable)]
#[repr(C)]
pub struct RelVector3 {
    pub x: F32,
    pub y: F32,
    pub z: F32,
}

unsafe impl<R: Region> Emplace<RelVector3, R> for &'_ data::Vector3 {
    fn emplaced_meta(&self) -> <RelVector3 as ptr_meta::Pointee>::Metadata {}

    unsafe fn emplace_unsized_unchecked(
        self,
        out: In<Slot<'_, RelVector3>, R>,
    ) {
        munge!(
            let RelVector3 {
                x,
                y,
                z,
            } = out;
        );

        self.x.emplace(x);
        self.y.emplace(y);
        self.z.emplace(z);
    }
}

#[derive(DropRaw, Move, Portable)]
#[repr(C)]
pub struct RelTriangle {
    pub v0: RelVector3,
    pub v1: RelVector3,
    pub v2: RelVector3,
    pub normal: RelVector3,
}

unsafe impl<R: Region> Emplace<RelTriangle, R> for &'_ data::Triangle {
    fn emplaced_meta(&self) -> <RelTriangle as ptr_meta::Pointee>::Metadata {}

    unsafe fn emplace_unsized_unchecked(
        self,
        out: In<Slot<'_, RelTriangle>, R>,
    ) {
        munge!(
            let RelTriangle {
                v0,
                v1,
                v2,
                normal,
            } = out;
        );

        self.v0.emplace(v0);
        self.v1.emplace(v1);
        self.v2.emplace(v2);
        self.normal.emplace(normal);
    }
}

#[derive(DropRaw, Move, Portable)]
#[repr(C)]
pub struct RelMesh<A: RawRegionalAllocator> {
    pub triangles: RelVec<RelTriangle, A>,
}

unsafe impl<A, R> Emplace<RelMesh<A>, R::Region> for FromData<'_, R, data::Mesh>
where
    A: DropRaw + RawRegionalAllocator<Region = R::Region>,
    R: RegionalAllocator + RelAllocator<A, R::Region>,
{
    fn emplaced_meta(&self) -> <RelMesh<A> as ptr_meta::Pointee>::Metadata {}

    unsafe fn emplace_unsized_unchecked(
        self,
        out: In<Slot<'_, RelMesh<A>>, A::Region>,
    ) {
        use ::rel_alloc::vec;

        munge!(let RelMesh { triangles } = out);

        let triangles =
            vec::WithCapacity(self.alloc, self.data.triangles.len())
                .emplace_mut(triangles);

        RelVec::extend(In::into_inner(triangles), self.data.triangles.iter());
    }
}

fn populate_buffer_external<'a>(
    data: &data::Mesh,
    buffer: Slot<'a, [u8]>,
) -> usize {
    runtime_token!(AllocatorToken);
    lease_static!(AllocatorToken => Allocator: External<'static, Slab>);

    let mut allocator_token = AllocatorToken::acquire();
    let buffer = unsafe { ::core::mem::transmute(buffer) };
    let allocator = StaticVal::<Allocator>::new(
        &mut allocator_token,
        External::new(buffer).unwrap(),
    );

    StaticToken::acquire(|mut token| {
        let alloc = Brand::new_deref(allocator.as_ref(), &mut token);

        ::core::mem::forget(
            FromData { alloc, data }.emplace_in::<RelMesh<_>>(alloc),
        );

        alloc.inner().control().len()
    })
}

fn populate_buffer_prefix(data: &data::Mesh, buffer: Slot<'_, [u8]>) -> usize {
    StaticToken::acquire(|mut token| {
        let alloc =
            Prefix::<Slab, _>::try_new_in_region(buffer, &mut token).unwrap();

        ::core::mem::forget(
            FromData { alloc, data }
                .emplace_in::<RelMesh<RelPrefix<Slab, _>>>(alloc),
        );

        alloc.control().len()
    })
}

pub fn make_benches(
    rng: &mut impl Rng,
    input_size: usize,
) -> Benchmarks<data::Mesh> {
    Benchmarks {
        input: data::Mesh {
            triangles: generate_vec(rng, input_size),
        },
        bytes: Align16::frame(100 * input_size),
        benches: &[
            Benchmark {
                name: "populate_buffer_external",
                bench: populate_buffer_external,
            },
            Benchmark {
                name: "populate_buffer_prefix",
                bench: populate_buffer_prefix,
            },
        ],
    }
}
