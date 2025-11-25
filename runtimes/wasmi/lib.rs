#![crate_type = "dylib"]

use benchmark_utils::{
    BenchInstance, BenchRuntime, ExecuteTestFilter, ModuleImportsIter, TestFilter, elapsed_ms,
};
pub use wasmi::CompilationMode;

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
    _instance: wasmi::Instance,
    func: wasmi::TypedFunc<i64, i64>,
}

impl BenchRuntime for Wasmi {
    fn name(&self) -> &'static str {
        match (self.compilation_mode, self.validation) {
            (CompilationMode::Eager, Validation::Checked) => "wasmi.eager.checked",
            (CompilationMode::Eager, Validation::Unchecked) => "wasmi.eager.unchecked",
            (CompilationMode::LazyTranslation, Validation::Checked) => {
                "wasmi.lazy-translation.checked"
            }
            (CompilationMode::LazyTranslation, Validation::Unchecked) => {
                "wasmi.lazy-translation.unchecked"
            }
            (CompilationMode::Lazy, Validation::Checked) => "wasmi.lazy.checked",
            (CompilationMode::Lazy, Validation::Unchecked) => "wasmi.lazy.unchecked",
        }
    }

    fn test_filter(&self) -> TestFilter {
        // We are not interested in `unchecked` or `lazy-translation` execution benchmarks
        // since we do not expect them to have significantly different behavior compared to
        // `eager.checked` and `lazy.checked`.
        let execute = matches!(self.validation, Validation::Checked)
            && matches!(self.compilation_mode, CompilationMode::Eager);
        TestFilter {
            execute: ExecuteTestFilter::set_to(execute),
            ..TestFilter::set_to(true)
        }
    }

    fn compile(&self, wasm: &[u8], _imports: ModuleImportsIter) {
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
            _instance: instance,
            func,
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
}
