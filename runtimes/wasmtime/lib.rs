#![crate_type = "dylib"]

use benchmark_utils::{
    BenchInstance, BenchRuntime, CompileTestId, ExecuteTestId, TestId, elapsed_ms,
};

pub enum Strategy {
    Cranelift,
    Winch,
    Pulley,
}

pub struct Wasmtime {
    pub strategy: Strategy,
}

struct WasmtimeRuntime {
    store: wasmtime::Store<()>,
    _instance: wasmtime::Instance,
    func: wasmtime::TypedFunc<i64, i64>,
}

impl BenchRuntime for Wasmtime {
    fn name(&self) -> &'static str {
        match self.strategy {
            Strategy::Cranelift => "wasmtime.cranelift",
            Strategy::Winch => "wasmtime.winch",
            Strategy::Pulley => "wasmtime.pulley",
        }
    }

    fn can_run(&self, id: TestId) -> bool {
        match self.strategy {
            Strategy::Cranelift => match id {
                // Note: ffmpeg takes too long to compile for Cranelift
                TestId::Compile(CompileTestId::Ffmpeg) => false,
                _ => true,
            },
            Strategy::Winch => match id {
                // Note: winch does not support the Wasm `tail-call` proposal.
                TestId::Execute(ExecuteTestId::FibonacciTail) => false,
                _ => {
                    // Note: winch only works on `x86_64` and `aarch64`.
                    cfg!(target_arch = "x86_64") || cfg!(target_arch = "aarch64")
                }
            },
            Strategy::Pulley => match id {
                // Note: ffmpeg takes too long to compile for Pulley
                TestId::Compile(CompileTestId::Ffmpeg) => false,
                _ => true,
            },
        }
    }

    fn compile(&self, wasm: &[u8]) {
        let store = self.store();
        wasmtime::Module::new(store.engine(), wasm).unwrap();
    }

    fn load(&self, wasm: &[u8]) -> Box<dyn BenchInstance> {
        let mut store = self.store();
        let engine = store.engine();
        let module = wasmtime::Module::new(engine, wasm).unwrap();
        let linker = wasmtime::Linker::new(engine);
        let instance = linker.instantiate(&mut store, &module).unwrap();
        let func = instance
            .get_typed_func::<i64, i64>(&mut store, "run")
            .unwrap();
        Box::new(WasmtimeRuntime {
            store,
            _instance: instance,
            func,
        })
    }

    fn coremark(&self, wasm: &[u8]) -> f32 {
        let mut store = self.store();
        let mut linker = wasmtime::Linker::new(store.engine());
        linker
            .func_wrap("env", "clock_ms", elapsed_ms)
            .expect("Wasmtime: failed to define `clock_ms` host function");
        let module = wasmtime::Module::new(store.engine(), wasm)
            .expect("Wasmtime: failed to compile and validate coremark Wasm binary");
        linker
            .instantiate(&mut store, &module)
            .expect("Wasmtime: failed to instantiate coremark Wasm module")
            .get_typed_func::<(), f32>(&mut store, "run")
            .expect("Wasmtime: could not find \"run\" function export")
            .call(&mut store, ())
            .expect("Wasmtime: failed to execute \"run\" function")
    }
}

impl Wasmtime {
    fn store(&self) -> wasmtime::Store<()> {
        let mut config = wasmtime::Config::default();
        if matches!(self.strategy, Strategy::Cranelift) {
            config.wasm_tail_call(true);
        }
        config.strategy(match self.strategy {
            Strategy::Cranelift | Strategy::Pulley => wasmtime::Strategy::Cranelift,
            Strategy::Winch => wasmtime::Strategy::Winch,
        });
        if matches!(self.strategy, Strategy::Pulley) {
            config.target("pulley64").unwrap();
        }
        let engine = wasmtime::Engine::new(&config).unwrap();
        <wasmtime::Store<()>>::new(&engine, ())
    }
}

impl BenchInstance for WasmtimeRuntime {
    fn call(&mut self, input: i64) {
        self.func.call(&mut self.store, input).unwrap();
    }
}
