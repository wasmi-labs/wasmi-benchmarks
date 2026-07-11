#![crate_type = "dylib"]

use benchmark_utils as utils;
use benchmark_utils::{BenchInstance, BenchRuntime, TestId, elapsed_ms};
pub use wasmi::CompilationMode;
use wasmi::{Func, Val};

pub struct Wasmi {
    pub compilation_mode: CompilationMode,
    pub validation: Validation,
}

#[derive(Debug, Copy, Clone)]
pub enum Validation {
    Checked,
    Unchecked,
}

struct WasmiRuntime {
    store: wasmi::Store<()>,
    instance: wasmi::Instance,
    func: wasmi::TypedFunc<i64, i64>,
    params: Vec<Val>,
    results: Vec<Val>,
}

impl BenchRuntime for Wasmi {
    fn name(&self) -> &'static str {
        match (self.compilation_mode, self.validation) {
            (CompilationMode::Eager, Validation::Checked) => "wasmi-v2.eager.checked",
            (CompilationMode::Eager, Validation::Unchecked) => "wasmi-v2.eager.unchecked",
            (CompilationMode::LazyTranslation, Validation::Checked) => {
                "wasmi-v2.lazy-translation.checked"
            }
            (CompilationMode::LazyTranslation, Validation::Unchecked) => {
                "wasmi-v2.lazy-translation.unchecked"
            }
            (CompilationMode::Lazy, Validation::Checked) => "wasmi-v2.lazy.checked",
            (CompilationMode::Lazy, Validation::Unchecked) => "wasmi-v2.lazy.unchecked",
        }
    }

    fn can_run(&self, id: TestId) -> bool {
        match id {
            TestId::Execute(_) => {
                matches!(self.validation, Validation::Checked)
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
        let linker = wasmi::Linker::new(engine);
        let instance = linker.instantiate_and_start(&mut store, &module).unwrap();
        let func = instance.get_typed_func::<i64, i64>(&store, "run").unwrap();
        Box::new(WasmiRuntime {
            store,
            instance,
            func,
            params: Vec::new(),
            results: Vec::new(),
        })
    }

    fn coremark(&self, wasm: &[u8]) -> f32 {
        let mut store = <wasmi::Store<()>>::default();
        let mut linker = wasmi::Linker::new(store.engine());
        linker.func_wrap("env", "clock_ms", elapsed_ms).unwrap();
        let module = wasmi::Module::new(store.engine(), wasm).unwrap();
        linker
            .instantiate_and_start(&mut store, &module)
            .unwrap()
            .get_typed_func::<(), f32>(&mut store, "run")
            .unwrap()
            .call(&mut store, ())
            .unwrap()
    }
}

impl Wasmi {
    fn store(&self) -> wasmi::Store<()> {
        let mut config = wasmi::Config::default();
        config.wasm_tail_call(true);
        config.compilation_mode(self.compilation_mode);
        let engine = wasmi::Engine::new(&config);
        <wasmi::Store<()>>::new(&engine, ())
    }

    fn module(&self, engine: &wasmi::Engine, wasm: &[u8]) -> wasmi::Module {
        match self.validation {
            Validation::Checked => wasmi::Module::new(engine, wasm).unwrap(),
            Validation::Unchecked => {
                // SAFETY: We only use properly valid Wasm in our benchmarks.
                unsafe { wasmi::Module::new_unchecked(engine, wasm).unwrap() }
            }
        }
    }
}

impl BenchInstance for WasmiRuntime {
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
        assert_eq!(params.len(), func.ty(&self.store).params().len());
        assert_eq!(results.len(), func.ty(&self.store).results().len());
        self.prepare_params(params);
        self.prepare_results(&func);
        func.call(&mut self.store, &self.params[..], &mut self.results[..])?;
        self.write_back_results(results);
        Ok(())
    }
}

impl WasmiRuntime {
    fn prepare_params(&mut self, params: &[utils::Val]) {
        self.params.clear();
        self.params
            .extend(params.iter().copied().map(from_utils_val));
    }

    fn prepare_results(&mut self, func: &Func) {
        self.results.clear();
        for ty in func.ty(&self.store).results() {
            self.results.push(Val::default_for_ty(*ty))
        }
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
        utils::Val::F32(val) => Val::F32(val.into()),
        utils::Val::F64(val) => Val::F64(val.into()),
    }
}

fn into_utils_val(val: Val) -> utils::Val {
    match val {
        Val::I32(val) => utils::Val::I32(val),
        Val::I64(val) => utils::Val::I64(val),
        Val::F32(val) => utils::Val::F32(val.to_float()),
        Val::F64(val) => utils::Val::F64(val.to_float()),
        _ => panic!(),
    }
}
