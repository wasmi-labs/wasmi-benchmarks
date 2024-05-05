use super::{BenchRuntime, BenchVm};
use crate::utils::{ExecuteTestFilter, TestFilter};

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
                execute: ExecuteTestFilter {
                    fib_tailrec: false,
                    ..Default::default()
                },
                ..Default::default()
            },
            WasmerCompiler::Singlepass => TestFilter {
                execute: ExecuteTestFilter {
                    fib_tailrec: false,
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    fn compile(&self, wasm: &[u8]) {
        let mut store = self.store();
        wasmer::Module::new(&store, &wasm[..]).unwrap();
    }

    fn load(&self, wasm: &[u8]) -> Box<dyn BenchRuntime> {
        let mut store = self.store();
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

impl Wasmer {
    fn store(&self) -> wasmer::Store {
        match self.compiler {
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
        }
    }
}

impl BenchRuntime for WasmerRuntime {
    fn call(&mut self, input: i64) {
        self.func.call(&mut self.store, input).unwrap();
    }
}
