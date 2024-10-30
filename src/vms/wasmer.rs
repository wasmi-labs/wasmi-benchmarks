use super::{elapsed_ms, BenchRuntime, BenchVm};
use crate::utils::{ExecuteTestFilter, TestFilter};
use wasmi_new::ModuleImportsIter;

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

    fn compile(&self, wasm: &[u8], _imports: ModuleImportsIter) {
        let store = self.store();
        wasmer::Module::new(&store, wasm).unwrap();
    }

    fn load(&self, wasm: &[u8]) -> Box<dyn BenchRuntime> {
        let mut store = self.store();
        let module = wasmer::Module::new(&store, wasm).unwrap();
        let import_object = wasmer::imports! {
             "fluentbase_v1preview" => {
                "_write" => wasmer::Function::new_typed(&mut store, |offset: u32, length: u32,| {

                         }),
                "_exit" => wasmer::Function::new_typed(&mut store, |exit_code: i32| {

                         }),
            }
        };
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
