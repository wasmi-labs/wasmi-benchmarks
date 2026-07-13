use benchmark_utils::ExecuteTestId;
use benchmark_utils::{InputEncoding, Val, read_benchmark_file, wat2wasm};
use core::fmt;
use core::slice;
use criterion::Criterion;
use wasmi_benchmarks::vms_under_test;

/// Generic utility benchmark function for Wasm functions of type: T -> T
fn execute_benchmark<T>(c: &mut Criterion, input: T, encoding: InputEncoding, id: ExecuteTestId)
where
    T: Into<Val> + Copy + fmt::Display,
{
    execute_benchmark_with_val(c, input.into(), encoding, id)
}

/// Non-generic utility benchmark function for Wasm functions of type: T -> T
fn execute_benchmark_with_val(
    c: &mut Criterion,
    input: Val,
    encoding: InputEncoding,
    id: ExecuteTestId,
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
        1_000_000,
        InputEncoding::Wat,
        ExecuteTestId::CounterLocal,
    )
}

pub fn bench_counter_param(c: &mut Criterion) {
    execute_benchmark::<i32>(
        c,
        1_000_000,
        InputEncoding::Wat,
        ExecuteTestId::CounterParam,
    )
}

pub fn bench_fibonacci_rec(c: &mut Criterion) {
    execute_benchmark::<i64>(c, 30, InputEncoding::Wat, ExecuteTestId::FibonacciRec)
}

pub fn bench_fibonacci_iter(c: &mut Criterion) {
    execute_benchmark::<i64>(
        c,
        2_000_000,
        InputEncoding::Wat,
        ExecuteTestId::FibonacciIter,
    )
}

pub fn bench_fibonacci_tail(c: &mut Criterion) {
    execute_benchmark::<i64>(
        c,
        1_000_000,
        InputEncoding::Wat,
        ExecuteTestId::FibonacciTail,
    )
}

pub fn bench_primes(c: &mut Criterion) {
    execute_benchmark::<i64>(c, 1_000, InputEncoding::Wat, ExecuteTestId::Primes)
}

pub fn bench_matrix_multiply(c: &mut Criterion) {
    execute_benchmark::<i64>(c, 200, InputEncoding::Wat, ExecuteTestId::MatrixMultiply)
}

pub fn bench_argon2(c: &mut Criterion) {
    execute_benchmark::<i64>(c, 1, InputEncoding::Wasm, ExecuteTestId::Argon2)
}

pub fn bench_bulk_ops(c: &mut Criterion) {
    execute_benchmark::<i64>(c, 5_000, InputEncoding::Wat, ExecuteTestId::BulkOps)
}
