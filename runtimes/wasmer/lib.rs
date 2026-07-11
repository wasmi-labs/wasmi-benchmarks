#![crate_type = "dylib"]

use benchmark_utils::{BenchInstance, BenchRuntime, ExecuteTestId, TestId, elapsed_ms};

pub struct Wasmer {
    pub compiler: WasmerCompiler,
}

pub enum WasmerCompiler {
    Cranelift,
    Singlepass,
}

struct WasmerRuntime {
    store: wasmer::Store,
    _instance: wasmer::Instance,
    func: wasmer::TypedFunction<i64, i64>,
}

impl BenchRuntime for Wasmer {
    fn name(&self) -> &'static str {
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
        let func = instance
            .exports
            .get_typed_function::<i64, i64>(&store, "run")
            .unwrap();
        Box::new(WasmerRuntime {
            store,
            _instance: instance,
            func,
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
    fn call(&mut self, input: i64) {
        self.func.call(&mut self.store, input).unwrap();
    }
}
