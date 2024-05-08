use criterion::measurement::WallTime;
use criterion::{BenchmarkGroup, Criterion};
use wasmi_benchmarks::{vms_under_test, wat2wasm, BenchVm};

fn run_counter(g: &mut BenchmarkGroup<WallTime>, vm: &dyn BenchVm, input: i64) {
    if !vm.test_filter().execute.counter {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wat/counter.wat");
    let name = vm.name();
    let id = format!("{name}/{input}");
    g.bench_function(&id, |b| {
        let wasm = wat2wasm(WASM);
        let mut runtime = vm.load(&wasm[..]);
        b.iter(|| {
            runtime.call(input);
        });
    });
}

pub fn bench_counter(c: &mut Criterion) {
    const INPUT: i64 = 1_000_000;
    let mut g = c.benchmark_group("execute/counter");
    for vm in vms_under_test() {
        run_counter(&mut g, &*vm, INPUT);
    }
}

fn run_fib_recursive(g: &mut BenchmarkGroup<WallTime>, vm: &dyn BenchVm, input: i64) {
    if !vm.test_filter().execute.fib_recursive {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wat/fib.recursive.wat");
    let name = vm.name();
    let id = format!("{name}/{input}");
    g.bench_function(&id, |b| {
        let wasm = wat2wasm(WASM);
        let mut runtime = vm.load(&wasm[..]);
        b.iter(|| {
            runtime.call(input);
        });
    });
}

pub fn bench_fib_recursive(c: &mut Criterion) {
    const INPUT: i64 = 30;
    let mut g = c.benchmark_group("execute/fib/recursive");
    for vm in vms_under_test() {
        run_fib_recursive(&mut g, &*vm, INPUT);
    }
}

fn run_fib_iterative(g: &mut BenchmarkGroup<WallTime>, vm: &dyn BenchVm, input: i64) {
    if !vm.test_filter().execute.fib_iterative {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wat/fib.iterative.wat");
    let name = vm.name();
    let id = format!("{name}/{input}");
    g.bench_function(&id, |b| {
        let wasm = wat2wasm(WASM);
        let mut runtime = vm.load(&wasm[..]);
        b.iter(|| {
            runtime.call(input);
        });
    });
}

pub fn bench_fib_iterative(c: &mut Criterion) {
    const INPUT: i64 = 2_000_000;
    let mut g = c.benchmark_group("execute/fib/iterative");
    for vm in vms_under_test() {
        run_fib_iterative(&mut g, &*vm, INPUT);
    }
}

fn run_fib_tailrec(g: &mut BenchmarkGroup<WallTime>, vm: &dyn BenchVm, input: i64) {
    if !vm.test_filter().execute.fib_tailrec {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wat/fib.tailrec.wat");
    let name = vm.name();
    let id = format!("{name}/{input}");
    g.bench_function(&id, |b| {
        let wasm = wat2wasm(WASM);
        let mut runtime = vm.load(&wasm[..]);
        b.iter(|| {
            runtime.call(input);
        });
    });
}

pub fn bench_fib_tailrec(c: &mut Criterion) {
    const INPUT: i64 = 1_000_000;
    let mut g = c.benchmark_group("execute/fib/tailrec");
    for vm in vms_under_test() {
        run_fib_tailrec(&mut g, &*vm, INPUT);
    }
}

fn run_primes(g: &mut BenchmarkGroup<WallTime>, vm: &dyn BenchVm, input: i64) {
    if !vm.test_filter().execute.primes {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wat/primes.wat");
    let name = vm.name();
    let id = format!("{name}/{input}");
    g.bench_function(&id, |b| {
        let wasm = wat2wasm(WASM);
        let mut runtime = vm.load(&wasm[..]);
        b.iter(|| {
            runtime.call(input);
        });
    });
}

pub fn bench_primes(c: &mut Criterion) {
    const INPUT: i64 = 1_000;
    let mut g = c.benchmark_group("execute/primes");
    for vm in vms_under_test() {
        run_primes(&mut g, &*vm, INPUT);
    }
}

fn run_matrix_multiply(g: &mut BenchmarkGroup<WallTime>, vm: &dyn BenchVm, input: i64) {
    if !vm.test_filter().execute.matrix_multiply {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wat/matrix-multiplication.wat");
    let name = vm.name();
    let id = format!("{name}/{input}");
    g.bench_function(&id, |b| {
        let wasm = wat2wasm(WASM);
        let mut runtime = vm.load(&wasm[..]);
        b.iter(|| {
            runtime.call(input);
        });
    });
}

pub fn bench_matrix_multiply(c: &mut Criterion) {
    const INPUT: i64 = 200;
    let mut g = c.benchmark_group("execute/matmul");
    for vm in vms_under_test() {
        run_matrix_multiply(&mut g, &*vm, INPUT);
    }
}
