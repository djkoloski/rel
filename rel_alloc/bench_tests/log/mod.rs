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
use ::rel_alloc::{alloc::RelAllocator, EmplaceIn, RelString, RelVec};
use ::rel_allocators::{
    brand::Brand,
    external::External,
    prefix::{Prefix, RelPrefix},
    slab::Slab,
};
use ::rel_core::{Emplace, EmplaceExt, Move, Portable, U16, U64};
use ::rel_util::Align16;
use ::situ::{alloc::RawRegionalAllocator, DropRaw};

use crate::{benchmarks::*, from_data::FromData, gen::generate_vec};

#[derive(DropRaw, Move, Portable)]
#[repr(C)]
pub struct RelAddress {
    pub x0: u8,
    pub x1: u8,
    pub x2: u8,
    pub x3: u8,
}

unsafe impl<R: Region> Emplace<RelAddress, R> for &'_ data::Address {
    fn emplaced_meta(&self) -> <RelAddress as ptr_meta::Pointee>::Metadata {}

    unsafe fn emplace_unsized_unchecked(
        self,
        out: In<Slot<'_, RelAddress>, R>,
    ) {
        munge!(
            let RelAddress {
                x0,
                x1,
                x2,
                x3,
            } = out;
        );

        self.x0.emplace(x0);
        self.x1.emplace(x1);
        self.x2.emplace(x2);
        self.x3.emplace(x3);
    }
}

#[derive(DropRaw, Move, Portable)]
#[repr(C)]
pub struct RelEntry<A: RawRegionalAllocator> {
    pub address: RelAddress,
    pub identity: RelString<A>,
    pub userid: RelString<A>,
    pub date: RelString<A>,
    pub request: RelString<A>,
    pub code: U16,
    pub size: U64,
}

unsafe impl<A, R> Emplace<RelEntry<A>, R::Region>
    for FromData<'_, R, data::Entry>
where
    A: DropRaw + RawRegionalAllocator<Region = R::Region>,
    R: Clone + RegionalAllocator + RelAllocator<A, R::Region>,
{
    fn emplaced_meta(&self) -> <RelEntry<A> as ptr_meta::Pointee>::Metadata {}

    unsafe fn emplace_unsized_unchecked(
        self,
        out: In<Slot<'_, RelEntry<A>>, A::Region>,
    ) {
        use ::rel_alloc::string;

        munge!(
            let RelEntry {
                address,
                identity,
                userid,
                date,
                request,
                code,
                size,
            } = out;
        );

        let Self { alloc, data } = self;

        data.address.emplace(address);
        string::Clone(alloc.clone(), &data.identity).emplace(identity);
        string::Clone(alloc.clone(), &data.userid).emplace(userid);
        string::Clone(alloc.clone(), &data.date).emplace(date);
        string::Clone(alloc.clone(), &data.request).emplace(request);
        data.code.emplace(code);
        data.size.emplace(size);
    }
}

#[derive(DropRaw, Move, Portable)]
#[repr(C)]
pub struct RelLog<A: RawRegionalAllocator> {
    pub entries: RelVec<RelEntry<A>, A>,
}

unsafe impl<A, R> Emplace<RelLog<A>, R::Region> for FromData<'_, R, data::Log>
where
    A: DropRaw + Move<R::Region> + RawRegionalAllocator<Region = R::Region>,
    R: Clone + RegionalAllocator + RelAllocator<A, R::Region>,
{
    fn emplaced_meta(&self) -> <RelLog<A> as ptr_meta::Pointee>::Metadata {}

    unsafe fn emplace_unsized_unchecked(
        self,
        out: In<Slot<'_, RelLog<A>>, A::Region>,
    ) {
        use ::rel_alloc::vec;

        munge!(let RelLog { entries } = out);

        let entries =
            vec::WithCapacity(self.alloc.clone(), self.data.entries.len())
                .emplace_mut(entries);

        RelVec::extend(
            In::into_inner(entries),
            self.data.entries.iter().map(|data| FromData {
                alloc: self.alloc.clone(),
                data,
            }),
        );
    }
}

fn populate_buffer_external(data: &data::Log, buffer: Slot<'_, [u8]>) -> usize {
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
            FromData { alloc, data }.emplace_in::<RelLog<_>>(alloc),
        );

        alloc.inner().control().len()
    })
}

fn populate_buffer_prefix(data: &data::Log, buffer: Slot<'_, [u8]>) -> usize {
    StaticToken::acquire(|mut token| {
        let alloc =
            Prefix::<Slab, _>::try_new_in_region(buffer, &mut token).unwrap();

        ::core::mem::forget(
            FromData { alloc, data }
                .emplace_in::<RelLog<RelPrefix<Slab, _>>>(alloc),
        );

        alloc.control().len()
    })
}

pub fn make_benches(
    rng: &mut impl Rng,
    input_size: usize,
) -> Benchmarks<data::Log> {
    Benchmarks {
        input: data::Log {
            entries: generate_vec(rng, input_size),
        },
        bytes: Align16::frame(1_000 * input_size),
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
