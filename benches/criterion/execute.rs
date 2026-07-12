use benchmark_utils::ExecuteTestId;
use benchmark_utils::{InputEncoding, Val, read_benchmark_file, wat2wasm};
use core::fmt;
use criterion::Criterion;
use wasmi_benchmarks::vms_under_test;

/// Generic utility benchmark function for Wasm functions of type: T -> T
fn execute_benchmark<T>(
    c: &mut Criterion,
    name: &str,
    input: T,
    encoding: InputEncoding,
    id: ExecuteTestId,
) where
    T: Into<Val> + Copy + fmt::Display,
{
    execute_benchmark_with_val(c, name, input.into(), encoding, id)
}

/// Non-generic utility benchmark function for Wasm functions of type: T -> T
fn execute_benchmark_with_val(
    c: &mut Criterion,
    name: &str,
    input: Val,
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
            let input_ty = input.ty();
            let inputs = [input.into()];
            let mut results = [Val::default_for_ty(input_ty)];
            b.iter(|| {
                runtime
                    .call_with("run", &inputs[..], &mut results[..])
                    .unwrap();
            });
        });
    }
}

pub fn bench_counter_local(c: &mut Criterion) {
    execute_benchmark::<i32>(
        c,
        "counter-local",
        1_000_000,
        InputEncoding::Wat,
        ExecuteTestId::CounterLocal,
    )
}

pub fn bench_counter_param(c: &mut Criterion) {
    execute_benchmark::<i32>(
        c,
        "counter-param",
        1_000_000,
        InputEncoding::Wat,
        ExecuteTestId::CounterParam,
    )
}

pub fn bench_fibonacci_rec(c: &mut Criterion) {
    execute_benchmark::<i64>(
        c,
        "fibonacci-rec",
        30,
        InputEncoding::Wat,
        ExecuteTestId::FibonacciRec,
    )
}

pub fn bench_fibonacci_iter(c: &mut Criterion) {
    execute_benchmark::<i64>(
        c,
        "fibonacci-iter",
        2_000_000,
        InputEncoding::Wat,
        ExecuteTestId::FibonacciIter,
    )
}

pub fn bench_fibonacci_tail(c: &mut Criterion) {
    execute_benchmark::<i64>(
        c,
        "fibonacci-tail",
        1_000_000,
        InputEncoding::Wat,
        ExecuteTestId::FibonacciTail,
    )
}

pub fn bench_primes(c: &mut Criterion) {
    execute_benchmark::<i64>(
        c,
        "primes",
        1_000,
        InputEncoding::Wat,
        ExecuteTestId::Primes,
    )
}

pub fn bench_matrix_multiply(c: &mut Criterion) {
    execute_benchmark::<i64>(
        c,
        "matmul",
        200,
        InputEncoding::Wat,
        ExecuteTestId::MatrixMultiply,
    )
}

pub fn bench_argon2(c: &mut Criterion) {
    execute_benchmark::<i64>(c, "argon2", 1, InputEncoding::Wasm, ExecuteTestId::Argon2)
}

pub fn bench_bulk_ops(c: &mut Criterion) {
    execute_benchmark::<i64>(
        c,
        "bulk-ops",
        5_000,
        InputEncoding::Wat,
        ExecuteTestId::BulkOps,
    )
}
