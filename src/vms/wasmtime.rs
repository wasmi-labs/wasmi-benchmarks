use super::{BenchRuntime, BenchVm};
use crate::utils::{CompileTestFilter, ExecuteTestFilter, TestFilter};
use wasmi_new::ModuleImportsIter;

pub struct Wasmtime {
    pub strategy: wasmtime::Strategy,
}

struct WasmtimeRuntime {
    store: wasmtime::Store<()>,
    _instance: wasmtime::Instance,
    func: wasmtime::TypedFunc<i64, i64>,
}

impl BenchVm for Wasmtime {
    fn name(&self) -> &'static str {
        match self.strategy {
            wasmtime::Strategy::Cranelift | wasmtime::Strategy::Auto => "wasmtime.cranelift",
            wasmtime::Strategy::Winch => "wasmtime.winch",
            _ => panic!("unknown Wasmtime strategy"),
        }
    }

    fn test_filter(&self) -> TestFilter {
        match self.strategy {
            wasmtime::Strategy::Auto | wasmtime::Strategy::Cranelift => {
                TestFilter {
                    compile: CompileTestFilter {
                        ffmpeg: false, // takes too long to compile
                        ..Default::default()
                    },
                    ..Default::default()
                }
            }
            wasmtime::Strategy::Winch => {
                let winch_works = cfg!(target_arch = "x86_64");
                TestFilter {
                    execute: ExecuteTestFilter {
                        fib_tailrec: false,
                        ..ExecuteTestFilter::set_to(winch_works)
                    },
                    ..TestFilter::set_to(winch_works)
                }
            }
            unknown => panic!("unknown Wasmtime strategy: {unknown:?}"),
        }
    }

    fn compile(&self, wasm: &[u8], _imports: ModuleImportsIter) {
        let store = self.store();
        wasmtime::Module::new(store.engine(), wasm).unwrap();
    }

    fn load(&self, wasm: &[u8]) -> Box<dyn BenchRuntime> {
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
            _instance: instance,
            func,
        })
    }
}

impl Wasmtime {
    fn store(&self) -> wasmtime::Store<()> {
        let mut config = wasmtime::Config::default();
        config.wasm_tail_call(true);
        config.strategy(self.strategy);
        let engine = wasmtime::Engine::new(&config).unwrap();
        <wasmtime::Store<()>>::new(&engine, ())
    }
}

impl BenchRuntime for WasmtimeRuntime {
    fn call(&mut self, input: i64) {
        self.func.call(&mut self.store, input).unwrap();
    }
}
