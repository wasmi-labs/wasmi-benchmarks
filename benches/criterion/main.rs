mod compile;
mod execute;
mod execute_with_jit;

use criterion::{criterion_group, criterion_main, Criterion};
use std::time::Duration;

criterion_main!(bench_execute, bench_compile, bench_execute_jit);
criterion_group!(
    name = bench_execute;
    config = Criterion::default()
        .sample_size(10)
        .measurement_time(Duration::from_millis(2000))
        .warm_up_time(Duration::from_millis(1000));
    targets =
        execute::bench_counter,
        execute::bench_fib_recursive,
        execute::bench_fib_iterative,
        execute::bench_fib_tailrec,
        execute::bench_primes,
        execute::bench_matrix_multiply,
        execute::bench_argon2,
        execute::bench_bulk_ops,
);
criterion_group!(
    name = bench_compile;
    config = Criterion::default()
        .sample_size(10)
        .measurement_time(Duration::from_millis(2000))
        .warm_up_time(Duration::from_millis(1000));
    targets =
        compile::bench_bz2,
        compile::bench_pulldown_cmark,
        compile::bench_spidermonkey,
        compile::bench_ffmpeg,
        compile::bench_coremark_minimal,
        compile::bench_argon2,
        compile::bench_erc20,
);
criterion_group!(
    name = bench_execute_jit;
    config = Criterion::default()
        .sample_size(10)
        .measurement_time(Duration::from_millis(2000))
        .warm_up_time(Duration::from_millis(1000));
    targets =
        execute_with_jit::bench_greeting,
);