mod execute;
mod startup;

use criterion::{Criterion, criterion_group, criterion_main};
use std::time::Duration;

criterion_main!(bench_execute, bench_startup);
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
    name = bench_startup;
    config = Criterion::default()
        .sample_size(10)
        .measurement_time(Duration::from_millis(2000))
        .warm_up_time(Duration::from_millis(1000));
    targets =
        startup::bench_bz2,
        startup::bench_pulldown_cmark,
        startup::bench_spidermonkey,
        startup::bench_ffmpeg,
        startup::bench_coremark_minimal,
        startup::bench_argon2,
        startup::bench_erc20,
);
