mod execute;
mod startup;

use self::execute::bench_execute;
use self::startup::bench_startup;
use criterion::criterion_main;

criterion_main!(bench_execute, bench_startup);
