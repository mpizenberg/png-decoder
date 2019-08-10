use criterion::criterion_main;

mod benchmarks;

criterion_main! {
    benchmarks::decode_file::benches,
    benchmarks::unfilter::benches,
}
