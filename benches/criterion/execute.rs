use criterion::Criterion;
use wasmi_benchmarks::{InputEncoding, TestFilter, read_benchmark_file, vms_under_test, wat2wasm};

fn execute_benchmark(
    c: &mut Criterion,
    name: &str,
    input: i64,
    encoding: InputEncoding,
    filter: impl Fn(&TestFilter) -> bool,
) {
    let wasm = read_benchmark_file(encoding, name);
    let mut g = c.benchmark_group(format!("execute/{name}"));
    for vm in vms_under_test() {
        if !filter(&vm.test_filter()) {
            continue;
        }
        let id = format!("{}/{}", vm.name(), input);
        g.bench_function(&id, |b| {
            let wasm = wat2wasm(&wasm[..]);
            let mut runtime = vm.load(&wasm[..]);
            b.iter(|| {
                runtime.call(input);
            });
        });
    }
}

pub fn bench_counter(c: &mut Criterion) {
    execute_benchmark(c, "counter", 1_000_000, InputEncoding::Wat, |filter| {
        filter.execute.counter
    })
}

pub fn bench_fib_recursive(c: &mut Criterion) {
    execute_benchmark(c, "fib.recursive", 30, InputEncoding::Wat, |filter| {
        filter.execute.fib_recursive
    })
}

pub fn bench_fib_iterative(c: &mut Criterion) {
    execute_benchmark(
        c,
        "fib.iterative",
        2_000_000,
        InputEncoding::Wat,
        |filter| filter.execute.fib_iterative,
    )
}

pub fn bench_fib_tailrec(c: &mut Criterion) {
    execute_benchmark(c, "fib.tailrec", 1_000_000, InputEncoding::Wat, |filter| {
        filter.execute.fib_tailrec
    })
}

pub fn bench_primes(c: &mut Criterion) {
    execute_benchmark(c, "primes", 1_000, InputEncoding::Wat, |filter| {
        filter.execute.primes
    })
}

pub fn bench_matrix_multiply(c: &mut Criterion) {
    execute_benchmark(c, "matmul", 200, InputEncoding::Wat, |filter| {
        filter.execute.matrix_multiply
    })
}

pub fn bench_argon2(c: &mut Criterion) {
    execute_benchmark(c, "argon2", 1, InputEncoding::Wasm, |filter| {
        filter.execute.argon2
    })
}

pub fn bench_bulk_ops(c: &mut Criterion) {
    execute_benchmark(c, "bulk-ops", 5_000, InputEncoding::Wat, |filter| {
        filter.execute.bulk_ops
    })
}
