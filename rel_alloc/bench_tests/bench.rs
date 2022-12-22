pub mod benchmarks;
pub mod from_data;
pub mod gen;
mod log;
mod mc_savedata;
mod mesh;

use ::criterion::{criterion_group, criterion_main, Criterion};

fn run_benchmarks<I>(
    c: &mut Criterion,
    group_name: &'static str,
    mut benchmarks: benchmarks::Benchmarks<I>,
) {
    let mut group = c.benchmark_group(group_name);
    for benchmark in benchmarks.benches {
        let size = benchmark.run(&benchmarks.input, &mut benchmarks.bytes);
        println!("{}/{}", group_name, benchmark.name);
        println!("                        size:   {} bytes", size);
        group.bench_function(benchmark.name, |b| {
            b.iter(|| benchmark.run(&benchmarks.input, &mut benchmarks.bytes))
        });
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    run_benchmarks(
        c,
        "mesh",
        mesh::make_benches(&mut gen::default_rng(), 125_000),
    );

    run_benchmarks(
        c,
        "log",
        log::make_benches(&mut gen::default_rng(), 10_000),
    );

    run_benchmarks(
        c,
        "mc_savedata",
        mc_savedata::make_benches(&mut gen::default_rng(), 500),
    );
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
