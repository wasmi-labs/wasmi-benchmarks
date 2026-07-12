#![crate_type = "dylib"]

use benchmark_utils::{self as utils, ExecuteTestId};
use benchmark_utils::{BenchInstance, BenchRuntime, TestId, elapsed_ms};
pub use wasm3::CompilationMode;
use wasm3::{Func, Val};

pub struct Wasm3 {
    pub compilation_mode: CompilationMode,
}

struct Wasm3Runtime {
    store: wasm3::Store<()>,
    instance: wasm3::Instance,
    func: wasm3::TypedFunc<i64, i64>,
    params: Vec<Val>,
    results: Vec<Val>,
}

impl BenchRuntime for Wasm3 {
    fn name(&self) -> &'static str {
        match self.compilation_mode {
            CompilationMode::Eager => "wasm3.eager",
            CompilationMode::Lazy => "wasm3.lazy",
        }
    }

    fn can_run(&self, id: TestId) -> bool {
        match id {
            TestId::Execute(_) => {
                !matches!(id, TestId::Execute(ExecuteTestId::FibonacciTail))
                    && matches!(self.compilation_mode, CompilationMode::Eager)
            }
            _ => true,
        }
    }

    fn compile(&self, wasm: &[u8]) {
        let store = self.store();
        self.module(store.engine(), wasm);
    }

    fn load(&self, wasm: &[u8]) -> Box<dyn BenchInstance> {
        let mut store = self.store();
        let engine = store.engine();
        let module = self.module(engine, wasm);
        let linker = wasm3::Linker::new(engine);
        let instance = linker.instantiate_and_start(&mut store, &module).unwrap();
        let func = instance.get_typed_func::<i64, i64>(&store, "run").unwrap();
        Box::new(Wasm3Runtime {
            store,
            instance,
            func,
            params: Vec::new(),
            results: Vec::new(),
        })
    }

    fn coremark(&self, wasm: &[u8]) -> f32 {
        let mut store = <wasm3::Store<()>>::default();
        let mut linker = wasm3::Linker::new(store.engine());
        linker.func_wrap("env", "clock_ms", elapsed_ms).unwrap();
        let module = wasm3::Module::new(store.engine(), wasm).unwrap();
        linker
            .instantiate_and_start(&mut store, &module)
            .unwrap()
            .get_typed_func::<(), f32>(&mut store, "run")
            .unwrap()
            .call(&mut store, ())
            .unwrap()
    }
}

impl Wasm3 {
    fn store(&self) -> wasm3::Store<()> {
        let mut config = wasm3::Config::default();
        config.compilation_mode(self.compilation_mode);
        let engine = wasm3::Engine::new(&config);
        <wasm3::Store<()>>::new(&engine, ())
    }

    fn module(&self, engine: &wasm3::Engine, wasm: &[u8]) -> wasm3::Module {
        wasm3::Module::new(engine, wasm).unwrap()
    }
}

impl BenchInstance for Wasm3Runtime {
    fn call(&mut self, input: i64) {
        self.func.call(&mut self.store, input).unwrap();
    }

    fn call_with(
        &mut self,
        name: &str,
        params: &[utils::Val],
        results: &mut [utils::Val],
    ) -> anyhow::Result<()> {
        let Some(func) = self.instance.get_func(&self.store, name) else {
            anyhow::bail!("failed to find function")
        };
        assert_eq!(params.len(), func.ty(&self.store)?.params().len());
        assert_eq!(results.len(), func.ty(&self.store)?.results().len());
        self.prepare_params(params);
        self.prepare_results(&func)?;
        func.call(&mut self.store, &self.params[..], &mut self.results[..])?;
        self.write_back_results(results);
        Ok(())
    }
}

impl Wasm3Runtime {
    fn prepare_params(&mut self, params: &[utils::Val]) {
        self.params.clear();
        self.params
            .extend(params.iter().copied().map(from_utils_val));
    }

    fn prepare_results(&mut self, func: &Func) -> anyhow::Result<()> {
        self.results.clear();
        for ty in func.ty(&self.store)?.results() {
            self.results.push(Val::default_for_ty(*ty))
        }
        Ok(())
    }

    fn write_back_results(&mut self, results: &mut [utils::Val]) {
        assert_eq!(results.len(), self.results.len());
        for (i, result) in results.iter_mut().enumerate() {
            let ty = self.results[i].ty();
            let src = core::mem::replace(&mut self.results[i], Val::default_for_ty(ty));
            *result = into_utils_val(src);
        }
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
    }
}
