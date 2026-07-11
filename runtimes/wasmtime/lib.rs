#![crate_type = "dylib"]

use benchmark_utils as utils;
use benchmark_utils::{
    BenchInstance, BenchRuntime, CompileTestId, ExecuteTestId, TestId, elapsed_ms,
};
use wasmtime::{Func, Val, ValType};

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
    instance: wasmtime::Instance,
    func: wasmtime::TypedFunc<i64, i64>,
    params: Vec<Val>,
    results: Vec<Val>,
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
            instance,
            func,
            params: Vec::new(),
            results: Vec::new(),
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

    fn call_with(
        &mut self,
        name: &str,
        params: &[utils::Val],
        results: &mut [utils::Val],
    ) -> anyhow::Result<()> {
        let Some(func) = self.instance.get_func(&mut self.store, name) else {
            anyhow::bail!("failed to find function")
        };
        assert_eq!(params.len(), func.ty(&self.store).params().len());
        assert_eq!(results.len(), func.ty(&self.store).results().len());
        self.prepare_params(params);
        self.prepare_results(&func);
        func.call(&mut self.store, &self.params[..], &mut self.results[..])?;
        self.write_back_results(results)?;
        Ok(())
    }
}

impl WasmtimeRuntime {
    fn prepare_params(&mut self, params: &[utils::Val]) {
        self.params.clear();
        self.params
            .extend(params.iter().copied().map(from_utils_val));
    }

    fn prepare_results(&mut self, func: &Func) {
        self.results.clear();
        for ty in func.ty(&self.store).results() {
            self.results.push(default_val(ty))
        }
    }

    fn write_back_results(&mut self, results: &mut [utils::Val]) -> anyhow::Result<()> {
        assert_eq!(results.len(), self.results.len());
        for (i, result) in results.iter_mut().enumerate() {
            let ty = self.results[i].ty(&self.store)?;
            let src = core::mem::replace(&mut self.results[i], default_val(ty));
            *result = into_utils_val(src);
        }
        Ok(())
    }
}

fn default_val(ty: ValType) -> Val {
    Val::default_for_ty(&ty).unwrap()
}

fn from_utils_val(val: utils::Val) -> Val {
    match val {
        utils::Val::I32(val) => Val::I32(val),
        utils::Val::I64(val) => Val::I64(val),
        utils::Val::F32(val) => Val::F32(val.to_bits()),
        utils::Val::F64(val) => Val::F64(val.to_bits()),
    }
}

fn into_utils_val(val: Val) -> utils::Val {
    match val {
        Val::I32(val) => utils::Val::I32(val),
        Val::I64(val) => utils::Val::I64(val),
        Val::F32(val) => utils::Val::F32(f32::from_bits(val)),
        Val::F64(val) => utils::Val::F64(f64::from_bits(val)),
        _ => panic!(),
    }
}
