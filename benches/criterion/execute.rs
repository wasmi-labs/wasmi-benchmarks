use benchmark_utils::ExecuteTestId;
use benchmark_utils::{InputEncoding, Val, read_benchmark_file, wat2wasm};
use core::fmt;
use core::slice;
use criterion::Criterion;
use wasmi_benchmarks::vms_under_test;

/// Generic utility benchmark function for Wasm functions of type: T -> T
fn execute_benchmark<T>(c: &mut Criterion, id: ExecuteTestId, input: T, encoding: InputEncoding)
where
    T: Into<Val> + Copy + fmt::Display,
{
    execute_benchmark_with_val(c, id, input.into(), encoding)
}

/// Non-generic utility benchmark function for Wasm functions of type: T -> T
fn execute_benchmark_with_val(
    c: &mut Criterion,
    id: ExecuteTestId,
    input: Val,
    encoding: InputEncoding,
) {
    let wasm = read_benchmark_file(encoding, id.into());
    let mut g = c.benchmark_group(format!("execute/{id}"));
    for vm in vms_under_test() {
        let Some(rt) = vm.setup(id.into()) else {
            continue;
        };
        let bench_id = format!("{}/{}", vm.id(), input);
        g.bench_function(&bench_id, |b| {
            // `load` consumes the runtime instance, so it runs once here rather than inside
            // the `bench_function` closure (which is `FnMut` and only times `call` via `b.iter`).
            let wasm = wat2wasm(&wasm[..]);
            let mut instance = rt.instantiate(&wasm[..]);
            let mut result = Val::default_for_ty(input.ty());
            b.iter(|| {
                instance
                    .call("run", slice::from_ref(&input), slice::from_mut(&mut result))
                    .unwrap();
            });
        });
    }
}

pub fn bench_counter_local(c: &mut Criterion) {
    execute_benchmark::<i32>(
        c,
        ExecuteTestId::CounterLocal,
        1_000_000,
        InputEncoding::Wat,
    )
}

pub fn bench_counter_param(c: &mut Criterion) {
    execute_benchmark::<i32>(
        c,
        ExecuteTestId::CounterParam,
        1_000_000,
        InputEncoding::Wat,
    )
}

pub fn bench_counter_global(c: &mut Criterion) {
    execute_benchmark::<i32>(
        c,
        ExecuteTestId::CounterGlobal,
        500_000,
        InputEncoding::Wat,
    )
}

pub fn bench_fibonacci_rec(c: &mut Criterion) {
    execute_benchmark::<i64>(c, ExecuteTestId::FibonacciRec, 30, InputEncoding::Wat)
}

pub fn bench_fibonacci_iter(c: &mut Criterion) {
    execute_benchmark::<i64>(
        c,
        ExecuteTestId::FibonacciIter,
        2_000_000,
        InputEncoding::Wat,
    )
}

pub fn bench_fibonacci_tail(c: &mut Criterion) {
    execute_benchmark::<i64>(
        c,
        ExecuteTestId::FibonacciTail,
        1_000_000,
        InputEncoding::Wat,
    )
}

pub fn bench_primes(c: &mut Criterion) {
    execute_benchmark::<i64>(c, ExecuteTestId::Primes, 1_000, InputEncoding::Wat)
}

pub fn bench_matrix_multiply(c: &mut Criterion) {
    execute_benchmark::<i64>(c, ExecuteTestId::MatrixMultiply, 200, InputEncoding::Wat)
}

pub fn bench_argon2(c: &mut Criterion) {
    execute_benchmark::<i64>(c, ExecuteTestId::Argon2, 1, InputEncoding::Wasm)
}

pub fn bench_bulk_ops(c: &mut Criterion) {
    execute_benchmark::<i64>(c, ExecuteTestId::BulkOps, 5_000, InputEncoding::Wat)
}
