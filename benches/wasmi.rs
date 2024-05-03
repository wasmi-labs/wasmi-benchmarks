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

#[derive(Debug, Copy, Clone)]
pub struct TestFilter {
    pub fib_iterative: bool,
    pub fib_recursive: bool,
    pub fib_tailrec: bool,
    pub primes: bool,
}

impl Default for TestFilter {
    fn default() -> Self {
        Self {
            fib_iterative: true,
            fib_recursive: true,
            fib_tailrec: true,
            primes: true,
        }
    }
}

/// Converts the `.wat` encoded `bytes` into `.wasm` encoded bytes.
pub fn wat2wasm(bytes: &[u8]) -> Vec<u8> {
    wat::parse_bytes(bytes).unwrap().into_owned()
}

pub trait BenchVm {
    fn name(&self) -> &'static str;
    fn test_filter(&self) -> TestFilter {
        TestFilter::default()
    }
    fn load(&self, wasm: &[u8]) -> Box<dyn BenchRuntime>;
}

pub trait BenchRuntime {
    fn call(&mut self, input: i64);
}

pub struct WasmiOld;

pub struct WasmiOldRuntime {
    store: wasmi_old::Store<()>,
    instance: wasmi_old::Instance,
    func: wasmi_old::TypedFunc<i64, i64>,
}

impl BenchVm for WasmiOld {
    fn name(&self) -> &'static str {
        "wasmi-v0.31"
    }

    fn load(&self, wasm: &[u8]) -> Box<dyn BenchRuntime> {
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
        Box::new(WasmiOldRuntime {
            store,
            instance,
            func,
        })
    }
}

impl BenchRuntime for WasmiOldRuntime {
    fn call(&mut self, input: i64) {
        self.func.call(&mut self.store, input).unwrap();
    }
}

pub struct WasmiNew {
    compilation_mode: wasmi_new::CompilationMode,
    validation: Validation,
}

#[derive(Debug, Copy, Clone)]
enum Validation {
    Checked,
    Unchecked,
}

pub struct WasmiNewRuntime {
    store: wasmi_new::Store<()>,
    instance: wasmi_new::Instance,
    func: wasmi_new::TypedFunc<i64, i64>,
}

impl BenchVm for WasmiNew {
    fn name(&self) -> &'static str {
        match (self.compilation_mode, self.validation) {
            (wasmi_new::CompilationMode::Eager, Validation::Checked) => "wasmi-v0.32.eager.checked",
            (wasmi_new::CompilationMode::Eager, Validation::Unchecked) => {
                "wasmi-v0.32.eager.unchecked"
            }
            (wasmi_new::CompilationMode::LazyTranslation, Validation::Checked) => {
                "wasmi-v0.32.lazy-translation.checked"
            }
            (wasmi_new::CompilationMode::LazyTranslation, Validation::Unchecked) => {
                "wasmi-v0.32.lazy-translation.unchecked"
            }
            (wasmi_new::CompilationMode::Lazy, Validation::Checked) => "wasmi-v0.32.lazy.checked",
            (wasmi_new::CompilationMode::Lazy, Validation::Unchecked) => {
                "wasmi-v0.32.lazy.unchecked"
            }
        }
    }

    fn load(&self, wasm: &[u8]) -> Box<dyn BenchRuntime> {
        let mut config = wasmi_new::Config::default();
        config.wasm_tail_call(true);
        config.compilation_mode(self.compilation_mode);
        let engine = wasmi_new::Engine::new(&config);
        let mut store = <wasmi_new::Store<()>>::new(&engine, ());
        let module = match self.validation {
            Validation::Checked => wasmi_new::Module::new(&engine, &wasm[..]).unwrap(),
            Validation::Unchecked => {
                // SAFETY: We only use properly valid Wasm in our benchmarks.
                unsafe { wasmi_new::Module::new_unchecked(&engine, &wasm[..]).unwrap() }
            }
        };
        let linker = wasmi_new::Linker::new(&engine);
        let instance = linker
            .instantiate(&mut store, &module)
            .unwrap()
            .start(&mut store)
            .unwrap();
        let func = instance.get_typed_func::<i64, i64>(&store, "run").unwrap();
        Box::new(WasmiNewRuntime {
            store,
            instance,
            func,
        })
    }
}

impl BenchRuntime for WasmiNewRuntime {
    fn call(&mut self, input: i64) {
        self.func.call(&mut self.store, input).unwrap();
    }
}

pub struct Wasmtime {
    strategy: wasmtime::Strategy,
}

pub struct WasmtimeRuntime {
    store: wasmtime::Store<()>,
    instance: wasmtime::Instance,
    func: wasmtime::TypedFunc<i64, i64>,
}

impl BenchVm for Wasmtime {
    fn name(&self) -> &'static str {
        match self.strategy {
            wasmtime::Strategy::Cranelift | wasmtime::Strategy::Auto => "wasmtime.cranelift",
            wasmtime::Strategy::Winch => "wasmtime.winch",
            _ => panic!("unknown Wasmtime strategy"),
        }
    }

    fn test_filter(&self) -> TestFilter {
        match self.strategy {
            wasmtime::Strategy::Auto | wasmtime::Strategy::Cranelift => TestFilter::default(),
            wasmtime::Strategy::Winch => {
                let winch_works = cfg!(target_arch = "x86_64");
                TestFilter {
                    fib_iterative: winch_works,
                    fib_recursive: winch_works,
                    fib_tailrec: false,
                    primes: winch_works,
                }
            }
            unknown => panic!("unknown Wasmtime strategy: {unknown:?}"),
        }
    }

    fn load(&self, wasm: &[u8]) -> Box<dyn BenchRuntime> {
        let mut config = wasmtime::Config::default();
        config.wasm_tail_call(true);
        config.strategy(self.strategy);
        let engine = wasmtime::Engine::new(&config).unwrap();
        let mut store = <wasmtime::Store<()>>::new(&engine, ());
        let module = wasmtime::Module::new(&engine, &wasm[..]).unwrap();
        let linker = wasmtime::Linker::new(&engine);
        let instance = linker.instantiate(&mut store, &module).unwrap();
        let func = instance
            .get_typed_func::<i64, i64>(&mut store, "run")
            .unwrap();
        Box::new(WasmtimeRuntime {
            store,
            instance,
            func,
        })
    }
}

impl BenchRuntime for WasmtimeRuntime {
    fn call(&mut self, input: i64) {
        self.func.call(&mut self.store, input).unwrap();
    }
}

pub struct Tinywasm;

pub struct TinywasmRuntime {
    store: tinywasm::Store,
    instance: tinywasm::ModuleInstance,
    func: tinywasm::FuncHandleTyped<i64, i64>,
}

impl BenchVm for Tinywasm {
    fn name(&self) -> &'static str {
        "tinywasm"
    }

    fn test_filter(&self) -> TestFilter {
        TestFilter {
            fib_tailrec: false,
            ..Default::default()
        }
    }

    fn load(&self, wasm: &[u8]) -> Box<dyn BenchRuntime> {
        let mut store = tinywasm::Store::new();
        let module = tinywasm::Module::parse_bytes(&wasm[..]).unwrap();
        let instance = module.instantiate(&mut store, None).unwrap();
        let func = instance
            .exported_func::<i64, i64>(&mut store, "run")
            .unwrap();
        Box::new(TinywasmRuntime {
            store,
            instance,
            func,
        })
    }
}

impl BenchRuntime for TinywasmRuntime {
    fn call(&mut self, input: i64) {
        self.func.call(&mut self.store, input).unwrap();
    }
}

pub struct Wasmer {
    compiler: WasmerCompiler,
}

enum WasmerCompiler {
    Cranelift,
    Singlepass,
}

pub struct WasmerRuntime {
    store: wasmer::Store,
    instance: wasmer::Instance,
    func: wasmer::TypedFunction<i64, i64>,
}

impl BenchVm for Wasmer {
    fn name(&self) -> &'static str {
        match self.compiler {
            WasmerCompiler::Cranelift => "wasmer.cranelift",
            WasmerCompiler::Singlepass => "wasmer.singlepass",
        }
    }

    fn test_filter(&self) -> TestFilter {
        match self.compiler {
            WasmerCompiler::Cranelift => TestFilter {
                fib_tailrec: false,
                ..Default::default()
            },
            WasmerCompiler::Singlepass => TestFilter {
                fib_tailrec: false,
                ..Default::default()
            },
        }
    }

    fn load(&self, wasm: &[u8]) -> Box<dyn BenchRuntime> {
        let mut store = match self.compiler {
            WasmerCompiler::Cranelift => {
                let mut builder =
                    wasmer::sys::EngineBuilder::new(wasmer_compiler_cranelift::Cranelift::new());
                let mut features = wasmer::sys::Features::new();
                features.tail_call(true);
                let engine = builder.set_features(Some(features)).engine();
                wasmer::Store::new(engine)
            }
            WasmerCompiler::Singlepass => {
                wasmer::Store::new(wasmer_compiler_singlepass::Singlepass::new())
            }
        };
        let module = wasmer::Module::new(&store, &wasm[..]).unwrap();
        let import_object = wasmer::imports! {};
        let instance = wasmer::Instance::new(&mut store, &module, &import_object).unwrap();
        let func = instance
            .exports
            .get_typed_function::<i64, i64>(&mut store, "run")
            .unwrap();
        Box::new(WasmerRuntime {
            store,
            instance,
            func,
        })
    }
}

impl BenchRuntime for WasmerRuntime {
    fn call(&mut self, input: i64) {
        self.func.call(&mut self.store, input).unwrap();
    }
}

fn vms() -> Vec<Box<dyn BenchVm>> {
    vec![
        Box::new(WasmiOld),
        Box::new(WasmiNew {
            compilation_mode: wasmi_new::CompilationMode::Eager,
            validation: Validation::Checked,
        }),
        Box::new(WasmiNew {
            compilation_mode: wasmi_new::CompilationMode::Lazy,
            validation: Validation::Unchecked,
        }),
        Box::new(WasmiNew {
            compilation_mode: wasmi_new::CompilationMode::Lazy,
            validation: Validation::Checked,
        }),
        Box::new(WasmiNew {
            compilation_mode: wasmi_new::CompilationMode::Lazy,
            validation: Validation::Unchecked,
        }),
        Box::new(Tinywasm),
        Box::new(Wasmtime {
            strategy: wasmtime::Strategy::Cranelift,
        }),
        Box::new(Wasmtime {
            strategy: wasmtime::Strategy::Winch,
        }),
        Box::new(Wasmer {
            compiler: WasmerCompiler::Cranelift,
        }),
        Box::new(Wasmer {
            compiler: WasmerCompiler::Singlepass,
        }),
    ]
}

fn run_fib_recursive(c: &mut Criterion, vm: &dyn BenchVm, input: i64) {
    if !vm.test_filter().fib_recursive {
        return;
    }
    static FIB_REC_WAT: &[u8] = include_bytes!("../res/wat/fib.recursive.wat");
    let name = vm.name();
    let id = format!("fib/recursive/{name}/{input}");
    c.bench_function(&id, |b| {
        let wasm = wat2wasm(FIB_REC_WAT);
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
    static FIB_REC_WAT: &[u8] = include_bytes!("../res/wat/fib.iterative.wat");
    let name = vm.name();
    let id = format!("fib/iterative/{name}/{input}");
    c.bench_function(&id, |b| {
        let wasm = wat2wasm(FIB_REC_WAT);
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
    static FIB_REC_WAT: &[u8] = include_bytes!("../res/wat/fib.tailrec.wat");
    let name = vm.name();
    let id = format!("fib/tailrec/{name}/{input}");
    c.bench_function(&id, |b| {
        let wasm = wat2wasm(FIB_REC_WAT);
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
    static FIB_REC_WAT: &[u8] = include_bytes!("../res/wat/primes.wat");
    let name = vm.name();
    let id = format!("primes/{name}/{input}");
    c.bench_function(&id, |b| {
        let wasm = wat2wasm(FIB_REC_WAT);
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
