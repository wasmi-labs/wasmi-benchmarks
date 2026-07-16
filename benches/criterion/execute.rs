use benchmark_utils::{CallTyped as _, ExecuteTestId};
use benchmark_utils::{InputEncoding, Val, read_benchmark_file, wat2wasm};
use core::fmt;
use core::slice;
use criterion::{Criterion, criterion_group};
use std::time::Duration;
use wasmi_benchmarks::vms_under_test;

criterion_group!(
    name = bench_execute;
    config = Criterion::default()
        .sample_size(10)
        .measurement_time(Duration::from_millis(2000))
        .warm_up_time(Duration::from_millis(1000));
    targets =
        bench_counter_local,
        bench_counter_param,
        bench_counter_global,
        bench_fibonacci_rec,
        bench_fibonacci_iter,
        bench_fibonacci_tail,
        bench_sort,
        bench_execute_prime_sieve,
        bench_matrix_multiply,
        bench_nbody,
        bench_argon2,
        bench_tiny_keccak,

        bench_primes,
        bench_bulk_ops,
);

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

fn bench_counter_local(c: &mut Criterion) {
    execute_benchmark::<i32>(
        c,
        ExecuteTestId::CounterLocal,
        1_000_000,
        InputEncoding::Wat,
    )
}

fn bench_counter_param(c: &mut Criterion) {
    execute_benchmark::<i32>(
        c,
        ExecuteTestId::CounterParam,
        1_000_000,
        InputEncoding::Wat,
    )
}

fn bench_counter_global(c: &mut Criterion) {
    execute_benchmark::<i32>(c, ExecuteTestId::CounterGlobal, 500_000, InputEncoding::Wat)
}

fn bench_fibonacci_rec(c: &mut Criterion) {
    execute_benchmark::<i64>(c, ExecuteTestId::FibonacciRec, 30, InputEncoding::Wat)
}

fn bench_fibonacci_iter(c: &mut Criterion) {
    execute_benchmark::<i64>(
        c,
        ExecuteTestId::FibonacciIter,
        2_000_000,
        InputEncoding::Wat,
    )
}

fn bench_fibonacci_tail(c: &mut Criterion) {
    execute_benchmark::<i64>(
        c,
        ExecuteTestId::FibonacciTail,
        1_000_000,
        InputEncoding::Wat,
    )
}

fn bench_primes(c: &mut Criterion) {
    execute_benchmark::<i64>(c, ExecuteTestId::Primes, 1_000, InputEncoding::Wat)
}

fn bench_bulk_ops(c: &mut Criterion) {
    execute_benchmark::<i64>(c, ExecuteTestId::BulkOps, 5_000, InputEncoding::Wat)
}

fn bench_sort(c: &mut Criterion) {
    let id = ExecuteTestId::Sort;
    let wasm = read_benchmark_file(InputEncoding::RustCompiledWasm, id.into());
    let mut g = c.benchmark_group(format!("execute/{id}"));
    for vm in vms_under_test() {
        let Some(rt) = vm.setup(id.into()) else {
            continue;
        };
        let len: i32 = 1_000_000;
        let bench_id = format!("{}/{}", vm.id(), len);
        g.bench_function(&bench_id, |b| {
            let mut instance = rt.instantiate(&wasm[..]);
            let data = instance.call_typed::<i32, i32>("setup", len).unwrap();
            b.iter(|| {
                instance.call_typed::<i32, ()>("run", data).unwrap();
            });
            instance.call_typed::<i32, ()>("teardown", data).unwrap();
        });
    }
}

fn bench_execute_prime_sieve(c: &mut Criterion) {
    let id = ExecuteTestId::PrimeSieve;
    let wasm = read_benchmark_file(InputEncoding::RustCompiledWasm, id.into());
    let mut g = c.benchmark_group(format!("execute/{id}"));
    for vm in vms_under_test() {
        let Some(rt) = vm.setup(id.into()) else {
            continue;
        };
        let len: i64 = 10_000_000;
        let bench_id = format!("{}/{}", vm.id(), len);
        g.bench_function(&bench_id, |b| {
            let mut instance = rt.instantiate(&wasm[..]);
            let data = instance.call_typed::<i64, i32>("setup", len).unwrap();
            b.iter(|| {
                instance.call_typed::<i32, ()>("run", data).unwrap();
            });
            let len_primes = instance.call_typed::<i32, i64>("len_primes", data).unwrap();
            let largest_prime = instance
                .call_typed::<i32, i64>("largest_prime", data)
                .unwrap();
            assert_eq!(len_primes, 664579);
            assert_eq!(largest_prime, 9999991);
            instance.call_typed::<i32, ()>("teardown", data).unwrap();
        });
    }
}

fn bench_matrix_multiply(c: &mut Criterion) {
    let id = ExecuteTestId::MatrixMultiply;
    let wasm = read_benchmark_file(InputEncoding::RustCompiledWasm, id.into());
    let mut g = c.benchmark_group(format!("execute/{id}"));
    for vm in vms_under_test() {
        let Some(rt) = vm.setup(id.into()) else {
            continue;
        };
        let n: i32 = 400;
        let bench_id = format!("{}/{}", vm.id(), n);
        g.bench_function(&bench_id, |b| {
            let mut instance = rt.instantiate(&wasm[..]);
            let data = instance.call_typed::<i32, i32>("setup", n).unwrap();
            b.iter(|| {
                instance.call_typed::<i32, ()>("run", data).unwrap();
            });
            instance.call_typed::<i32, ()>("teardown", data).unwrap();
        });
    }
}

fn bench_nbody(c: &mut Criterion) {
    let id = ExecuteTestId::Nbody;
    let wasm = read_benchmark_file(InputEncoding::RustCompiledWasm, id.into());
    let mut g = c.benchmark_group(format!("execute/{id}"));
    for vm in vms_under_test() {
        let Some(rt) = vm.setup(id.into()) else {
            continue;
        };
        let n: i32 = 400;
        let bench_id = format!("{}/{}", vm.id(), n);
        g.bench_function(&bench_id, |b| {
            let mut instance = rt.instantiate(&wasm[..]);
            let data = instance.call_typed::<i32, i32>("setup", n).unwrap();
            b.iter(|| {
                instance.call_typed::<i32, ()>("run", data).unwrap();
            });
            instance.call_typed::<i32, ()>("teardown", data).unwrap();
        });
    }
}

fn bench_argon2(c: &mut Criterion) {
    let id = ExecuteTestId::Argon2;
    let wasm = read_benchmark_file(InputEncoding::RustCompiledWasm, id.into());
    let mut g = c.benchmark_group(format!("execute/{id}"));
    for vm in vms_under_test() {
        let Some(rt) = vm.setup(id.into()) else {
            continue;
        };
        let len: i32 = 10_000;
        let bench_id = format!("{}/{}", vm.id(), len);
        g.bench_function(&bench_id, |b| {
            let mut instance = rt.instantiate(&wasm[..]);
            let data = instance.call_typed::<i32, i32>("setup", len).unwrap();
            b.iter(|| {
                instance.call_typed::<i32, ()>("run", data).unwrap();
            });
            let output = instance.call_typed::<i32, i64>("output", data).unwrap();
            assert_eq!(output, 0x4CDBBC7DE0EAA94);
            instance.call_typed::<i32, ()>("teardown", data).unwrap();
        });
    }
}

fn bench_tiny_keccak(c: &mut Criterion) {
    let id = ExecuteTestId::TinyKeccak;
    let wasm = read_benchmark_file(InputEncoding::RustCompiledWasm, id.into());
    let mut g = c.benchmark_group(format!("execute/{id}"));
    for vm in vms_under_test() {
        let Some(rt) = vm.setup(id.into()) else {
            continue;
        };
        let bench_id = vm.id().to_string();
        g.bench_function(&bench_id, |b| {
            let mut instance = rt.instantiate(&wasm[..]);
            let data = instance.call_typed::<(), i32>("setup", ()).unwrap();
            b.iter(|| {
                instance.call_typed::<i32, ()>("run", data).unwrap();
            });
            instance.call_typed::<i32, ()>("teardown", data).unwrap();
        });
    }
}
