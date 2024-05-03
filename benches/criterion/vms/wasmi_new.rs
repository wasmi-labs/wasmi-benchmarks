use super::{BenchRuntime, BenchVm};

pub struct WasmiNew {
    pub compilation_mode: wasmi_new::CompilationMode,
    pub validation: Validation,
}

#[derive(Debug, Copy, Clone)]
pub enum Validation {
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
