#![crate_type = "dylib"]

use benchmark_utils as utils;
use benchmark_utils::{BenchInstance, BenchRuntime, ExecuteTestId, TestId, elapsed_ms};
use wasmer::Function;
use wasmer::Type as ValType;
use wasmer::Value as Val;

pub struct Wasmer {
    pub compiler: WasmerCompiler,
}

pub enum WasmerCompiler {
    Cranelift,
    Singlepass,
}

struct WasmerRuntime {
    store: wasmer::Store,
    instance: wasmer::Instance,
    params: Vec<Val>,
    results: Vec<Val>,
}

impl BenchRuntime for Wasmer {
    fn id(&self) -> &'static str {
        match self.compiler {
            WasmerCompiler::Cranelift => "wasmer.cranelift",
            WasmerCompiler::Singlepass => "wasmer.singlepass",
        }
    }

    fn can_run(&self, id: TestId) -> bool {
        !matches!(id, TestId::Execute(ExecuteTestId::FibonacciTail))
    }

    fn compile(&self, wasm: &[u8]) {
        let store = self.store();
        wasmer::Module::new(&store, wasm).unwrap();
    }

    fn load(&self, wasm: &[u8]) -> Box<dyn BenchInstance> {
        let mut store = self.store();
        let module = wasmer::Module::new(&store, wasm).unwrap();
        let import_object = wasmer::imports! {};
        let instance = wasmer::Instance::new(&mut store, &module, &import_object).unwrap();
        Box::new(WasmerRuntime {
            store,
            instance,
            params: Vec::new(),
            results: Vec::new(),
        })
    }

    fn coremark(&self, wasm: &[u8]) -> f32 {
        let mut store = self.store();
        let module = wasmer::Module::new(&store, wasm).unwrap();
        let import_object = wasmer::imports! {
            "env" => {
                "clock_ms" => wasmer::Function::new_typed(&mut store, elapsed_ms),
            }
        };
        let instance = wasmer::Instance::new(&mut store, &module, &import_object).unwrap();
        instance
            .exports
            .get_typed_function::<(), f32>(&store, "run")
            .unwrap()
            .call(&mut store)
            .unwrap()
    }
}

impl Wasmer {
    fn store(&self) -> wasmer::Store {
        match self.compiler {
            WasmerCompiler::Cranelift => {
                let builder =
                    wasmer::sys::EngineBuilder::new(wasmer_compiler_cranelift::Cranelift::new());
                let mut features = wasmer::sys::Features::new();
                features.tail_call(true);
                let engine = builder.set_features(Some(features)).engine();
                wasmer::Store::new(engine)
            }
            WasmerCompiler::Singlepass => {
                let builder =
                    wasmer::sys::EngineBuilder::new(wasmer_compiler_singlepass::Singlepass::new());
                wasmer::Store::new(builder)
            }
        }
    }
}

impl BenchInstance for WasmerRuntime {
    fn call_with(
        &mut self,
        name: &str,
        params: &[utils::Val],
        results: &mut [utils::Val],
    ) -> anyhow::Result<()> {
        let func = self.instance.exports.get_function(name).cloned()?;
        assert_eq!(params.len(), func.ty(&self.store).params().len());
        assert_eq!(results.len(), func.ty(&self.store).results().len());
        self.prepare_params(params);
        self.prepare_results(&func);
        let call_results = func.call(&mut self.store, &self.params[..])?;
        self.write_back_results(results, call_results);
        Ok(())
    }
}

impl WasmerRuntime {
    fn prepare_params(&mut self, params: &[utils::Val]) {
        self.params.clear();
        self.params
            .extend(params.iter().copied().map(from_utils_val));
    }

    fn prepare_results(&mut self, func: &Function) {
        self.results.clear();
        for ty in func.ty(&self.store).results() {
            self.results.push(default_val(*ty))
        }
    }

    fn write_back_results(&self, dst: &mut [utils::Val], src: Box<[Val]>) {
        assert_eq!(dst.len(), src.len());
        for (dst, src) in dst.iter_mut().zip(src) {
            *dst = into_utils_val(src);
        }
    }
}

fn default_val(ty: ValType) -> Val {
    match ty {
        ValType::I32 => Val::I32(0),
        ValType::I64 => Val::I64(0),
        ValType::F32 => Val::F32(0.0),
        ValType::F64 => Val::F64(0.0),
        _ => unreachable!(),
    }
}

fn from_utils_val(val: utils::Val) -> Val {
    match val {
        utils::Val::I32(val) => Val::I32(val),
        utils::Val::I64(val) => Val::I64(val),
        utils::Val::F32(val) => Val::F32(val),
        utils::Val::F64(val) => Val::F64(val),
    }
}

fn into_utils_val(val: Val) -> utils::Val {
    match val {
        Val::I32(val) => utils::Val::I32(val),
        Val::I64(val) => utils::Val::I64(val),
        Val::F32(val) => utils::Val::F32(val),
        Val::F64(val) => utils::Val::F64(val),
        _ => panic!(),
    }
}
