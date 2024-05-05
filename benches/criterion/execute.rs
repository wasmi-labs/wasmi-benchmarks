use crate::utils::{wat2wasm, TestFilter};
use crate::vms;
use crate::vms::{BenchRuntime, BenchVm};
use criterion::{criterion_group, criterion_main, Bencher, Criterion};
use std::time::Duration;

fn run_counter(c: &mut Criterion, vm: &dyn BenchVm, input: i64) {
    if !vm.test_filter().execute.counter {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wat/counter.wat");
    let name = vm.name();
    let id = format!("execute/counter/{name}/{input}");
    c.bench_function(&id, |b| {
        let wasm = wat2wasm(WASM);
        let mut runtime = vm.load(&wasm[..]);
        b.iter(|| {
            runtime.call(input);
        });
    });
}

pub fn bench_counter(c: &mut Criterion) {
    const INPUT: i64 = 1_000_000;
    for vm in vms() {
        run_counter(c, &*vm, INPUT);
    }
}

fn run_fib_recursive(c: &mut Criterion, vm: &dyn BenchVm, input: i64) {
    if !vm.test_filter().execute.fib_recursive {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wat/fib.recursive.wat");
    let name = vm.name();
    let id = format!("execute/fib/recursive/{name}/{input}");
    c.bench_function(&id, |b| {
        let wasm = wat2wasm(WASM);
        let mut runtime = vm.load(&wasm[..]);
        b.iter(|| {
            runtime.call(input);
        });
    });
}

pub fn bench_fib_recursive(c: &mut Criterion) {
    const INPUT: i64 = 30;
    for vm in vms() {
        run_fib_recursive(c, &*vm, INPUT);
    }
}

fn run_fib_iterative(c: &mut Criterion, vm: &dyn BenchVm, input: i64) {
    if !vm.test_filter().execute.fib_iterative {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wat/fib.iterative.wat");
    let name = vm.name();
    let id = format!("execute/fib/iterative/{name}/{input}");
    c.bench_function(&id, |b| {
        let wasm = wat2wasm(WASM);
        let mut runtime = vm.load(&wasm[..]);
        b.iter(|| {
            runtime.call(input);
        });
    });
}

pub fn bench_fib_iterative(c: &mut Criterion) {
    const INPUT: i64 = 2_000_000;
    for vm in vms() {
        run_fib_iterative(c, &*vm, INPUT);
    }
}

fn run_fib_tailrec(c: &mut Criterion, vm: &dyn BenchVm, input: i64) {
    if !vm.test_filter().execute.fib_tailrec {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wat/fib.tailrec.wat");
    let name = vm.name();
    let id = format!("execute/fib/tailrec/{name}/{input}");
    c.bench_function(&id, |b| {
        let wasm = wat2wasm(WASM);
        let mut runtime = vm.load(&wasm[..]);
        b.iter(|| {
            runtime.call(input);
        });
    });
}

pub fn bench_fib_tailrec(c: &mut Criterion) {
    const INPUT: i64 = 1_000_000;
    for vm in vms() {
        run_fib_tailrec(c, &*vm, INPUT);
    }
}

fn run_primes(c: &mut Criterion, vm: &dyn BenchVm, input: i64) {
    if !vm.test_filter().execute.primes {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wat/primes.wat");
    let name = vm.name();
    let id = format!("execute/primes/{name}/{input}");
    c.bench_function(&id, |b| {
        let wasm = wat2wasm(WASM);
        let mut runtime = vm.load(&wasm[..]);
        b.iter(|| {
            runtime.call(input);
        });
    });
}

pub fn bench_primes(c: &mut Criterion) {
    const INPUT: i64 = 1_000;
    for vm in vms() {
        run_primes(c, &*vm, INPUT);
    }
}

fn run_matrix_multiply(c: &mut Criterion, vm: &dyn BenchVm, input: i64) {
    if !vm.test_filter().execute.matrix_multiply {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wat/matrix-multiplication.wat");
    let name = vm.name();
    let id = format!("execute/matmul/{name}/{input}");
    c.bench_function(&id, |b| {
        let wasm = wat2wasm(WASM);
        let mut runtime = vm.load(&wasm[..]);
        b.iter(|| {
            runtime.call(input);
        });
    });
}

pub fn bench_matrix_multiply(c: &mut Criterion) {
    const INPUT: i64 = 200;
    for vm in vms() {
        run_matrix_multiply(c, &*vm, INPUT);
    }
}
