pub mod benchmarks;
pub mod from_data;
pub mod gen;
mod log;
mod mc_savedata;
mod mesh;

fn test_benchmarks<I>(mut benchmarks: benchmarks::Benchmarks<'_, I>) {
    for benchmark in benchmarks.benches {
        benchmark.run(&benchmarks.input, &mut benchmarks.bytes);
    }
}

#[test]
fn test_log_bench() {
    test_benchmarks(log::make_benches(&mut gen::default_rng(), 10));
}

#[test]
fn test_mesh_bench() {
    test_benchmarks(mesh::make_benches(&mut gen::default_rng(), 10));
}

#[test]
fn test_mc_savedata_bench() {
    test_benchmarks(mc_savedata::make_benches(&mut gen::default_rng(), 10));
}
