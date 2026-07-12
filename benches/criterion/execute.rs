use benchmark_utils::ExecuteTestId;
use benchmark_utils::{InputEncoding, read_benchmark_file, wat2wasm};
use criterion::Criterion;
use wasmi_benchmarks::vms_under_test;

fn execute_benchmark(
    c: &mut Criterion,
    name: &str,
    input: i64,
    encoding: InputEncoding,
    id: ExecuteTestId,
) {
    let wasm = read_benchmark_file(encoding, name);
    let mut g = c.benchmark_group(format!("execute/{name}"));
    for vm in vms_under_test() {
        if !vm.can_run(id.into()) {
            continue;
        }
        let id = format!("{}/{}", vm.id(), input);
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
    execute_benchmark(
        c,
        "counter",
        1_000_000,
        InputEncoding::Wat,
        ExecuteTestId::Counter,
    )
}

pub fn bench_fib_recursive(c: &mut Criterion) {
    execute_benchmark(
        c,
        "fib.recursive",
        30,
        InputEncoding::Wat,
        ExecuteTestId::FibonacciRec,
    )
}

pub fn bench_fib_iterative(c: &mut Criterion) {
    execute_benchmark(
        c,
        "fib.iterative",
        2_000_000,
        InputEncoding::Wat,
        ExecuteTestId::FibonacciIter,
    )
}

pub fn bench_fib_tailrec(c: &mut Criterion) {
    execute_benchmark(
        c,
        "fib.tailrec",
        1_000_000,
        InputEncoding::Wat,
        ExecuteTestId::FibonacciTail,
    )
}

pub fn bench_primes(c: &mut Criterion) {
    execute_benchmark(
        c,
        "primes",
        1_000,
        InputEncoding::Wat,
        ExecuteTestId::Primes,
    )
}

pub fn bench_matrix_multiply(c: &mut Criterion) {
    execute_benchmark(
        c,
        "matmul",
        200,
        InputEncoding::Wat,
        ExecuteTestId::MatrixMultiply,
    )
}

pub fn bench_argon2(c: &mut Criterion) {
    execute_benchmark(c, "argon2", 1, InputEncoding::Wasm, ExecuteTestId::Argon2)
}

pub fn bench_bulk_ops(c: &mut Criterion) {
    execute_benchmark(
        c,
        "bulk-ops",
        5_000,
        InputEncoding::Wat,
        ExecuteTestId::BulkOps,
    )
}
