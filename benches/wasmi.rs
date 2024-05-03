#![allow(unused)]

use criterion::{criterion_group, criterion_main, Bencher, Criterion};
use std::time::Duration;

criterion_group!(
    name = bench_wasmi;
    config = Criterion::default()
        .sample_size(10)
        .measurement_time(Duration::from_millis(2000))
        .warm_up_time(Duration::from_millis(1000));
    targets =
        bench_fib_recursive,
        bench_fib_iterative,
        bench_fib_tailrec,
        bench_primes,
);
criterion_main!(bench_wasmi);

/// Converts the `.wat` encoded `bytes` into `.wasm` encoded bytes.
pub fn wat2wasm(bytes: &[u8]) -> Vec<u8> {
    wat::parse_bytes(bytes).unwrap().into_owned()
}

pub trait BenchRunner {
    const NAME: &'static str;

    fn setup(wasm: &[u8]) -> Self;
    fn call(&mut self, input: i64);
}

pub struct WasmiOld {
    store: wasmi_old::Store<()>,
    instance: wasmi_old::Instance,
    func: wasmi_old::TypedFunc<i64, i64>,
}

impl BenchRunner for WasmiOld {
    const NAME: &'static str = "wasmi-v0.31";

    fn setup(wasm: &[u8]) -> Self {
        let mut config = wasmi_old::Config::default();
        config.wasm_tail_call(true);
        let engine = wasmi_old::Engine::new(&config);
        let mut store = <wasmi_old::Store<()>>::new(&engine, ());
        let module = wasmi_old::Module::new(&engine, &wasm[..]).unwrap();
        let linker = wasmi_old::Linker::new(&engine);
        let instance = linker
            .instantiate(&mut store, &module)
            .unwrap()
            .start(&mut store)
            .unwrap();
        let func = instance.get_typed_func::<i64, i64>(&store, "run").unwrap();
        Self {
            store,
            instance,
            func,
        }
    }

    fn call(&mut self, input: i64) {
        self.func.call(&mut self.store, input).unwrap();
    }
}

pub struct WasmiNew {
    store: wasmi_new::Store<()>,
    instance: wasmi_new::Instance,
    func: wasmi_new::TypedFunc<i64, i64>,
}

impl BenchRunner for WasmiNew {
    const NAME: &'static str = "wasmi-v0.32";

    fn setup(wasm: &[u8]) -> Self {
        let mut config = wasmi_new::Config::default();
        config.wasm_tail_call(true);
        let engine = wasmi_new::Engine::new(&config);
        let mut store = <wasmi_new::Store<()>>::new(&engine, ());
        let module = wasmi_new::Module::new(&engine, &wasm[..]).unwrap();
        let linker = wasmi_new::Linker::new(&engine);
        let instance = linker
            .instantiate(&mut store, &module)
            .unwrap()
            .start(&mut store)
            .unwrap();
        let func = instance.get_typed_func::<i64, i64>(&store, "run").unwrap();
        Self {
            store,
            instance,
            func,
        }
    }

    fn call(&mut self, input: i64) {
        self.func.call(&mut self.store, input).unwrap();
    }
}

pub struct Wasmtime {
    store: wasmtime::Store<()>,
    instance: wasmtime::Instance,
    func: wasmtime::TypedFunc<i64, i64>,
}

impl BenchRunner for Wasmtime {
    const NAME: &'static str = "wasmtime";

    fn setup(wasm: &[u8]) -> Self {
        let mut config = wasmtime::Config::default();
        config.wasm_tail_call(true);
        let engine = wasmtime::Engine::new(&config).unwrap();
        let mut store = <wasmtime::Store<()>>::new(&engine, ());
        let module = wasmtime::Module::new(&engine, &wasm[..]).unwrap();
        let linker = wasmtime::Linker::new(&engine);
        let instance = linker.instantiate(&mut store, &module).unwrap();
        let func = instance
            .get_typed_func::<i64, i64>(&mut store, "run")
            .unwrap();
        Self {
            store,
            instance,
            func,
        }
    }

    fn call(&mut self, input: i64) {
        self.func.call(&mut self.store, input).unwrap();
    }
}

pub struct Winch {
    store: wasmtime::Store<()>,
    instance: wasmtime::Instance,
    func: wasmtime::TypedFunc<i64, i64>,
}

impl BenchRunner for Winch {
    const NAME: &'static str = "winch";

    fn setup(wasm: &[u8]) -> Self {
        let mut config = wasmtime::Config::default();
        config.strategy(wasmtime::Strategy::Winch);
        config.wasm_tail_call(true);
        let engine = wasmtime::Engine::new(&config).unwrap();
        let mut store = <wasmtime::Store<()>>::new(&engine, ());
        let module = wasmtime::Module::new(&engine, &wasm[..]).unwrap();
        let linker = wasmtime::Linker::new(&engine);
        let instance = linker.instantiate(&mut store, &module).unwrap();
        let func = instance
            .get_typed_func::<i64, i64>(&mut store, "run")
            .unwrap();
        Self {
            store,
            instance,
            func,
        }
    }

    fn call(&mut self, input: i64) {
        self.func.call(&mut self.store, input).unwrap();
    }
}

pub struct Tinywasm {
    store: tinywasm::Store,
    instance: tinywasm::ModuleInstance,
    func: tinywasm::FuncHandleTyped<i64, i64>,
}

impl BenchRunner for Tinywasm {
    const NAME: &'static str = "tinywasm";

    fn setup(wasm: &[u8]) -> Self {
        let mut store = tinywasm::Store::new();
        let module = tinywasm::Module::parse_bytes(&wasm[..]).unwrap();
        let instance = module.instantiate(&mut store, None).unwrap();
        let func = instance
            .exported_func::<i64, i64>(&mut store, "run")
            .unwrap();
        Self {
            store,
            instance,
            func,
        }
    }

    fn call(&mut self, input: i64) {
        self.func.call(&mut self.store, input).unwrap();
    }
}
pub struct WasmerSinglepass {
    store: wasmer::Store,
    instance: wasmer::Instance,
    func: wasmer::TypedFunction<i64, i64>,
}

impl BenchRunner for WasmerSinglepass {
    const NAME: &'static str = "wasmer";

    fn setup(wasm: &[u8]) -> Self {
        let compiler = wasmer_compiler_singlepass::Singlepass::new();
        let mut store = wasmer::Store::new(compiler);
        let module = wasmer::Module::new(&store, &wasm[..]).unwrap();
        let import_object = wasmer::imports! {};
        let instance = wasmer::Instance::new(&mut store, &module, &import_object).unwrap();
        let func = instance
            .exports
            .get_typed_function::<i64, i64>(&mut store, "run")
            .unwrap();
        Self {
            store,
            instance,
            func,
        }
    }

    fn call(&mut self, input: i64) {
        self.func.call(&mut self.store, input).unwrap();
    }
}

fn run_fib_recursive<R>(c: &mut Criterion, input: i64)
where
    R: BenchRunner,
{
    static FIB_REC_WAT: &[u8] = include_bytes!("../res/wat/fib.recursive.wat");
    let name = R::NAME;
    let id = format!("fib/recursive/{name}/{input}");
    c.bench_function(&id, |b| {
        let expected = fib(input);
        let wasm = wat2wasm(FIB_REC_WAT);
        let mut runner = <R>::setup(&wasm[..]);
        b.iter(|| {
            runner.call(input);
        });
    });
}

fn bench_fib_recursive(c: &mut Criterion) {
    const INPUT: i64 = 30;
    run_fib_recursive::<WasmiOld>(c, INPUT);
    run_fib_recursive::<WasmiNew>(c, INPUT);
    run_fib_recursive::<Tinywasm>(c, INPUT);
    run_fib_recursive::<WasmerSinglepass>(c, INPUT);
    run_fib_recursive::<Wasmtime>(c, INPUT);
    // run_fib_recursive::<Winch>(c, INPUT);
}

fn run_fib_iterative<R>(c: &mut Criterion, input: i64)
where
    R: BenchRunner,
{
    static FIB_REC_WAT: &[u8] = include_bytes!("../res/wat/fib.iterative.wat");
    let name = R::NAME;
    let id = format!("fib/iterative/{name}/{input}");
    c.bench_function(&id, |b| {
        let expected = fib(input);
        let wasm = wat2wasm(FIB_REC_WAT);
        let mut runner = <R>::setup(&wasm[..]);
        b.iter(|| {
            runner.call(input);
        });
    });
}

fn bench_fib_iterative(c: &mut Criterion) {
    const INPUT: i64 = 2_000_000;
    run_fib_iterative::<WasmiOld>(c, INPUT);
    run_fib_iterative::<WasmiNew>(c, INPUT);
    run_fib_iterative::<Tinywasm>(c, INPUT);
    run_fib_iterative::<WasmerSinglepass>(c, INPUT);
    run_fib_iterative::<Wasmtime>(c, INPUT);
    // run_fib_iterative::<Winch>(c, INPUT);
}

fn run_fib_tailrec<R>(c: &mut Criterion, input: i64)
where
    R: BenchRunner,
{
    static FIB_REC_WAT: &[u8] = include_bytes!("../res/wat/fib.tailrec.wat");
    let name = R::NAME;
    let id = format!("fib/tailrec/{name}/{input}");
    c.bench_function(&id, |b| {
        let expected = fib(input);
        let wasm = wat2wasm(FIB_REC_WAT);
        let mut runner = <R>::setup(&wasm[..]);
        b.iter(|| {
            runner.call(input);
        });
    });
}

fn bench_fib_tailrec(c: &mut Criterion) {
    const INPUT: i64 = 1_000_000;
    run_fib_tailrec::<WasmiOld>(c, INPUT);
    run_fib_tailrec::<WasmiNew>(c, INPUT);
    // run_fib_tailrec::<Tinywasm>(c, INPUT);
    // run_fib_tailrec::<WasmerSinglepass>(c, INPUT);
    run_fib_tailrec::<Wasmtime>(c, INPUT);
    // run_fib_tailrec::<Winch>(c, INPUT);
}

fn run_primes<R>(c: &mut Criterion, input: i64)
where
    R: BenchRunner,
{
    static FIB_REC_WAT: &[u8] = include_bytes!("../res/wat/primes.wat");
    let name = R::NAME;
    let id = format!("primes/{name}/{input}");
    c.bench_function(&id, |b| {
        // let expected = primes(input);
        let wasm = wat2wasm(FIB_REC_WAT);
        let mut runner = <R>::setup(&wasm[..]);
        b.iter(|| {
            runner.call(input);
        });
    });
}

fn bench_primes(c: &mut Criterion) {
    const INPUT: i64 = 1_000;
    run_primes::<WasmiOld>(c, INPUT);
    run_primes::<WasmiNew>(c, INPUT);
    run_primes::<Tinywasm>(c, INPUT);
    run_primes::<WasmerSinglepass>(c, INPUT);
    run_primes::<Wasmtime>(c, INPUT);
    // run_primes::<Winch>(c, INPUT);
}
