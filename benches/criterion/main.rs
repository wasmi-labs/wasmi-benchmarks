#![allow(unused)]

mod utils;
mod vms;

use self::utils::{wat2wasm, TestFilter};
use self::vms::{BenchRuntime, BenchVm};
use criterion::{criterion_group, criterion_main, Bencher, Criterion};
use std::time::Duration;

criterion_group!(
    name = bench_wasmi;
    config = Criterion::default()
        .sample_size(10)
        .measurement_time(Duration::from_millis(2000))
        .warm_up_time(Duration::from_millis(1000));
    targets =
        bench_counter,
        bench_fib_recursive,
        bench_fib_iterative,
        bench_fib_tailrec,
        bench_primes,
        bench_matrix_multiply,
);
criterion_main!(bench_wasmi);

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

fn run_counter(c: &mut Criterion, vm: &dyn BenchVm, input: i64) {
    if !vm.test_filter().counter {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wat/counter.wat");
    let name = vm.name();
    let id = format!("counter/{name}/{input}");
    c.bench_function(&id, |b| {
        let wasm = wat2wasm(WASM);
        let mut runtime = vm.load(&wasm[..]);
        b.iter(|| {
            runtime.call(input);
        });
    });
}

fn bench_counter(c: &mut Criterion) {
    const INPUT: i64 = 1_000_000;
    for vm in vms() {
        run_counter(c, &*vm, INPUT);
    }
}

fn run_fib_recursive(c: &mut Criterion, vm: &dyn BenchVm, input: i64) {
    if !vm.test_filter().fib_recursive {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wat/fib.recursive.wat");
    let name = vm.name();
    let id = format!("fib/recursive/{name}/{input}");
    c.bench_function(&id, |b| {
        let wasm = wat2wasm(WASM);
        let mut runtime = vm.load(&wasm[..]);
        b.iter(|| {
            runtime.call(input);
        });
    });
}

fn bench_fib_recursive(c: &mut Criterion) {
    const INPUT: i64 = 30;
    for vm in vms() {
        run_fib_recursive(c, &*vm, INPUT);
    }
}

fn run_fib_iterative(c: &mut Criterion, vm: &dyn BenchVm, input: i64) {
    if !vm.test_filter().fib_iterative {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wat/fib.iterative.wat");
    let name = vm.name();
    let id = format!("fib/iterative/{name}/{input}");
    c.bench_function(&id, |b| {
        let wasm = wat2wasm(WASM);
        let mut runtime = vm.load(&wasm[..]);
        b.iter(|| {
            runtime.call(input);
        });
    });
}

fn bench_fib_iterative(c: &mut Criterion) {
    const INPUT: i64 = 2_000_000;
    for vm in vms() {
        run_fib_iterative(c, &*vm, INPUT);
    }
}

fn run_fib_tailrec(c: &mut Criterion, vm: &dyn BenchVm, input: i64) {
    if !vm.test_filter().fib_tailrec {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wat/fib.tailrec.wat");
    let name = vm.name();
    let id = format!("fib/tailrec/{name}/{input}");
    c.bench_function(&id, |b| {
        let wasm = wat2wasm(WASM);
        let mut runtime = vm.load(&wasm[..]);
        b.iter(|| {
            runtime.call(input);
        });
    });
}

fn bench_fib_tailrec(c: &mut Criterion) {
    const INPUT: i64 = 1_000_000;
    for vm in vms() {
        run_fib_tailrec(c, &*vm, INPUT);
    }
}

fn run_primes(c: &mut Criterion, vm: &dyn BenchVm, input: i64) {
    if !vm.test_filter().primes {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wat/primes.wat");
    let name = vm.name();
    let id = format!("primes/{name}/{input}");
    c.bench_function(&id, |b| {
        let wasm = wat2wasm(WASM);
        let mut runtime = vm.load(&wasm[..]);
        b.iter(|| {
            runtime.call(input);
        });
    });
}

fn bench_primes(c: &mut Criterion) {
    const INPUT: i64 = 1_000;
    for vm in vms() {
        run_primes(c, &*vm, INPUT);
    }
}

fn run_matrix_multiply(c: &mut Criterion, vm: &dyn BenchVm, input: i64) {
    if !vm.test_filter().matrix_multiply {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wat/matrix-multiplication.wat");
    let name = vm.name();
    let id = format!("matmul/{name}/{input}");
    c.bench_function(&id, |b| {
        let wasm = wat2wasm(WASM);
        let mut runtime = vm.load(&wasm[..]);
        b.iter(|| {
            runtime.call(input);
        });
    });
}

fn bench_matrix_multiply(c: &mut Criterion) {
    const INPUT: i64 = 200;
    for vm in vms() {
        run_matrix_multiply(c, &*vm, INPUT);
    }
}
