use super::{elapsed_ms, BenchRuntime, BenchVm};
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
                        argon2: false,
                        bulk_ops: false,
                        coremark: false,
                        ..ExecuteTestFilter::set_to(winch_works)
                    },
                    compile: CompileTestFilter {
                        argon2: false,
                        ..CompileTestFilter::set_to(winch_works)
                    },
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

    fn coremark(&self, wasm: &[u8]) -> f32 {
        let mut store = self.store();
        let mut linker = wasmtime::Linker::new(store.engine());
        linker
            .func_wrap("env", "clock_ms", elapsed_ms)
            .expect("Wasmtime: failed to define `clock_ms` host function");
        let module = wasmtime::Module::new(store.engine(), wasm)
            .expect("Wasmtime: failed to compile and validate coremark Wasm binary");
        linker
            .instantiate(&mut store, &module)
            .expect("Wasmtime: failed to instantiate coremark Wasm module")
            .get_typed_func::<(), f32>(&mut store, "run")
            .expect("Wasmtime: could not find \"run\" function export")
            .call(&mut store, ())
            .expect("Wasmtime: failed to execute \"run\" function")
    }
}

impl Wasmtime {
    fn store(&self) -> wasmtime::Store<()> {
        let mut config = wasmtime::Config::default();
        if matches!(self.strategy, wasmtime::Strategy::Auto | wasmtime::Strategy::Cranelift) {
            config.wasm_tail_call(true);
        }
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
