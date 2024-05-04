#![allow(unused)]

mod execute;
mod utils;
mod vms;

use self::utils::{wat2wasm, TestFilter};
use self::vms::{BenchRuntime, BenchVm};
use criterion::{criterion_group, criterion_main, Bencher, Criterion};
use std::time::Duration;

criterion_main!(bench_wasmi);
criterion_group!(
    name = bench_wasmi;
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
);

fn vms() -> Vec<Box<dyn BenchVm>> {
    use vms::wasmi_new::Validation;
    vec![
        Box::new(vms::WasmiOld),
        Box::new(vms::WasmiNew {
            compilation_mode: wasmi_new::CompilationMode::Eager,
            validation: Validation::Checked,
        }),
        Box::new(vms::WasmiNew {
            compilation_mode: wasmi_new::CompilationMode::Lazy,
            validation: Validation::Unchecked,
        }),
        Box::new(vms::WasmiNew {
            compilation_mode: wasmi_new::CompilationMode::Lazy,
            validation: Validation::Checked,
        }),
        Box::new(vms::Tinywasm),
        Box::new(vms::Wasmtime {
            strategy: wasmtime::Strategy::Cranelift,
        }),
        Box::new(vms::Wasmtime {
            strategy: wasmtime::Strategy::Winch,
        }),
        Box::new(vms::Wasmer {
            compiler: vms::wasmer::WasmerCompiler::Cranelift,
        }),
        Box::new(vms::Wasmer {
            compiler: vms::wasmer::WasmerCompiler::Singlepass,
        }),
    ]
}
