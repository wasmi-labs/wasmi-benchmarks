mod execute;
mod instantiate;

use criterion::{Criterion, criterion_group, criterion_main};
use std::time::Duration;

criterion_main!(bench_execute, bench_instantiate);
criterion_group!(
    name = bench_execute;
    config = Criterion::default()
        .sample_size(10)
        .measurement_time(Duration::from_millis(2000))
        .warm_up_time(Duration::from_millis(1000));
    targets =
        execute::bench_counter_local,
        execute::bench_counter_param,
        execute::bench_fibonacci_rec,
        execute::bench_fibonacci_iter,
        execute::bench_fibonacci_tail,
        execute::bench_primes,
        execute::bench_matrix_multiply,
        execute::bench_argon2,
        execute::bench_bulk_ops,
);
criterion_group!(
    name = bench_instantiate;
    config = Criterion::default()
        .sample_size(10)
        .measurement_time(Duration::from_millis(2000))
        .warm_up_time(Duration::from_millis(1000));
    targets =
        instantiate::bench_bz2,
        instantiate::bench_pulldown_cmark,
        instantiate::bench_spidermonkey,
        instantiate::bench_ffmpeg,
        instantiate::bench_coremark_minimal,
        instantiate::bench_argon2,
        instantiate::bench_erc20,
);
