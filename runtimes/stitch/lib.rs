#![crate_type = "dylib"]

use benchmark_utils as utils;
use benchmark_utils::{
    BenchInstance, BenchRuntime, CompileTestId, ExecuteTestId, TestId, elapsed_ms,
};
use core::slice;
use makepad_stitch::{Engine, ExternVal, Func, Instance, Linker, Module, Store, Val, ValType};

pub struct Stitch;

struct StitchRuntime {
    store: Store,
    instance: Instance,
    params: Vec<Val>,
    results: Vec<Val>,
}

impl BenchRuntime for Stitch {
    fn id(&self) -> &'static str {
        "stitch"
    }

    fn can_run(&self, id: TestId) -> bool {
        match id {
            TestId::Compile(id) => !matches!(id, CompileTestId::Ffmpeg),
            TestId::Execute(id) => {
                !matches!(id, ExecuteTestId::FibonacciTail | ExecuteTestId::Argon2)
            }
        }
    }

    fn compile(&self, wasm: &[u8]) {
        let engine = Engine::new();
        Module::new(&engine, wasm).unwrap();
    }

    fn load(&self, wasm: &[u8]) -> Box<dyn BenchInstance> {
        let engine = Engine::new();
        let mut store = Store::new(engine);
        let engine = store.engine();
        let module = Module::new(engine, wasm).unwrap();
        let linker = Linker::new();
        let instance = linker.instantiate(&mut store, &module).unwrap();
        Box::new(StitchRuntime {
            store,
            instance,
            params: Vec::new(),
            results: Vec::new(),
        })
    }

    fn coremark(&self, wasm: &[u8]) -> f32 {
        let engine = Engine::new();
        let mut store = Store::new(engine);
        let engine = store.engine();
        let module = Module::new(engine, wasm).unwrap();
        let mut linker = Linker::new();
        linker.define(
            "env",
            "clock_ms",
            ExternVal::Func(Func::wrap(&mut store, elapsed_ms)),
        );
        let instance = linker.instantiate(&mut store, &module).unwrap();
        let func = instance.exported_func("run").unwrap();
        let mut result = Val::F32(0.0);
        func.call(&mut store, &[], slice::from_mut(&mut result))
            .unwrap();
        result.to_f32().unwrap()
    }
}

impl BenchInstance for StitchRuntime {
    fn call(
        &mut self,
        name: &str,
        params: &[utils::Val],
        results: &mut [utils::Val],
    ) -> anyhow::Result<()> {
        let Some(func) = self.instance.exported_func(name) else {
            anyhow::bail!("failed to find function")
        };
        assert_eq!(params.len(), func.type_(&self.store).params().len());
        assert_eq!(results.len(), func.type_(&self.store).results().len());
        self.prepare_params(params);
        self.prepare_results(&func);
        func.call(&mut self.store, &self.params[..], &mut self.results[..])?;
        self.write_back_results(results);
        Ok(())
    }
}

impl StitchRuntime {
    fn prepare_params(&mut self, params: &[utils::Val]) {
        self.params.clear();
        self.params
            .extend(params.iter().copied().map(from_utils_val));
    }

    fn prepare_results(&mut self, func: &Func) {
        self.results.clear();
        for ty in func.type_(&self.store).results() {
            self.results.push(default_val(*ty))
        }
    }

    fn write_back_results(&self, results: &mut [utils::Val]) {
        assert_eq!(results.len(), self.results.len());
        for (i, result) in results.iter_mut().enumerate() {
            *result = into_utils_val(self.results[i]);
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
