use ::core::hint::black_box;
use ::mischief::{Frame, Slot};
use ::rel_util::Align16;

pub struct Benchmarks<'a, I> {
    pub input: I,
    pub bytes: Frame<Align16<[u8]>>,
    pub benches: &'a [Benchmark<I>],
}

pub struct Benchmark<I> {
    pub name: &'static str,
    pub bench: fn(&I, Slot<'_, [u8]>) -> usize,
}

impl<I> Benchmark<I> {
    pub fn run(&self, input: &I, bytes: &mut Frame<Align16<[u8]>>) -> usize {
        black_box((self.bench)(
            black_box(input),
            black_box(bytes.slot().as_bytes()),
        ))
    }
}
